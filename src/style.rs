use egui::{Color32, TextStyle};

use crate::style_sheet::{
    BACKGROUND_COLOR, BACKGROUND_CORNER_RADIUS, PROMPT_TEXT_COLOR, SELECTION_COLOR, TEXT_COLOR,
    TEXT_STYLE,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalStyle {
    pub background_color: Color32,
    pub background_corner_radius: f32,
    pub prompt_text_color: Color32,
    pub selection_color: Color32,
    pub text_color: Color32,
    pub text_style: TextStyle,
}

impl Default for TerminalStyle {
    fn default() -> Self {
        Self {
            background_color: BACKGROUND_COLOR,
            background_corner_radius: BACKGROUND_CORNER_RADIUS,
            prompt_text_color: PROMPT_TEXT_COLOR,
            selection_color: SELECTION_COLOR,
            text_color: TEXT_COLOR,
            text_style: TEXT_STYLE,
        }
    }
}
