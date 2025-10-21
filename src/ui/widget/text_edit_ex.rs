use eframe::egui::{CornerRadius, Frame, Response, TextEdit, Ui, Widget};
use eframe::epaint::Margin;

pub trait TextEditLikeButtonChain<'a> {
    fn like_button(self) -> TextEditLikeButton<'a>;
}

impl<'a> TextEditLikeButtonChain<'a> for TextEdit<'a> {
    #[inline]
    fn like_button(self) -> TextEditLikeButton<'a> {
        TextEditLikeButton { inner: self }
    }
}

pub struct TextEditLikeButton<'a> {
    inner: TextEdit<'a>,
}

impl<'a> Widget for TextEditLikeButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let v = ui.visuals().clone();
        let w = &v.widgets;
        let corner: CornerRadius = w.inactive.corner_radius;
        let fill = v
            .text_edit_bg_color
            .unwrap_or(w.inactive.bg_fill);

        let out = Frame {
            fill, //
            stroke: w.hovered.bg_stroke,
            inner_margin: Margin { left: 0, right: 0, top: 5, bottom: 5 },
            corner_radius: corner,
            ..Frame::default()
        }
        .show(ui, |ui| ui.add(self.inner.frame(false)));

        out.response.union(out.inner)
    }
}
