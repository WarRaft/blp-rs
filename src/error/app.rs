use crate::error::cause::Cause;
use crate::error::error::BlpError;
use crate::ui::viewer::update::App;
use fluent_templates::fluent_bundle::FluentArgs;
use std::fmt::Write;

impl App {
    /// Локализованный текст ошибки (с деревом причин).
    pub fn err_text_localized(&self, err: &BlpError) -> String {
        let mut out = String::new();
        self.fmt_err_loc(err, 0, &mut out);
        out
    }

    fn fmt_err_loc(&self, err: &BlpError, indent: usize, out: &mut String) {
        // Собираем FluentArgs из error.args
        let mut fargs = FluentArgs::new();
        for (k, v) in &err.args {
            fargs.set(*k, v.to_fluent_owned());
        }

        // Первая строка — локализованный заголовок
        for _ in 0..indent {
            let _ = write!(out, "  ");
        }
        let line = self.tr_args(err.key, &fargs);
        let _ = writeln!(out, "{line}");

        // Дочерние причины
        for cause in &err.causes {
            match cause {
                Cause::Blp(a) => self.fmt_err_loc(a, indent + 1, out),
                Cause::Std(e) => {
                    for _ in 0..(indent + 1) {
                        let _ = write!(out, "  ");
                    }
                    let _ = writeln!(out, "{e}");
                }
            }
        }
    }
}
