use crate::ui::i18n::lng_list::LngList;

pub fn detect_os_lang() -> LngList {
    let raw = sys_locale::get_locale().unwrap_or_default();
    let lc = raw.to_lowercase();
    let mut it = lc.split(|c| c == '-' || c == '_');

    let primary = it.next().unwrap_or("");

    // Быстрые пути для не-китайских
    match primary {
        "uk" => return LngList::Uk,
        "ru" => return LngList::Ru,
        // Иногда ОС/проги возвращают "sc"/"tc"
        "sc" => return LngList::Zh, // Simplified
        "tc" => return LngList::Tc, // Traditional
        // Кантонизский иногда как отдельный первичный язык
        "yue" => return LngList::Tc, // считаем как Traditional
        _ => {}
    }

    // Китайские варианты и их подметки
    if primary == "zh" {
        let subtags: Vec<&str> = it.collect();

        // Признаки традиционного китайского:
        // - script: Hant
        // - region: TW, HK, MO
        // - legacy: CHT
        // - диалект: yue (к примеру "zh-yue")
        let is_traditional = lc.contains("hant")
            || subtags
                .iter()
                .any(|s| matches!(*s, "tw" | "hk" | "mo"))
            || lc.contains("cht")
            || lc.contains("yue");

        if is_traditional {
            return LngList::Tc;
        }

        // Признаки упрощённого:
        // - script: Hans
        // - region: CN, SG
        // - legacy: CHS
        let is_simplified = lc.contains("hans")
            || subtags
                .iter()
                .any(|s| matches!(*s, "cn" | "sg"))
            || lc.contains("chs");

        if is_simplified {
            return LngList::Zh;
        }

        // Если просто "zh" или непонятные подметки — считаем упрощённым по умолчанию
        return LngList::Zh;
    }

    // Дефолт — английский
    LngList::En
}
