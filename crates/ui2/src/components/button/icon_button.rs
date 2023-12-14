use gpui::{AnyView, DefiniteLength};

use crate::prelude::*;
use crate::{ButtonCommon, ButtonLike, ButtonSize, ButtonStyle, Icon, IconSize};

use super::button_icon::ButtonIcon;

#[derive(IntoElement)]
pub struct IconButton {
    base: ButtonLike,
    icon: Icon,
    icon_size: IconSize,
    icon_color: Color,
    selected_icon: Option<Icon>,
    selected_style: Option<ButtonStyle>,
}

impl IconButton {
    pub fn new(id: impl Into<ElementId>, icon: Icon) -> Self {
        Self {
            base: ButtonLike::new(id),
            icon,
            icon_size: IconSize::default(),
            icon_color: Color::Default,
            selected_icon: None,
            selected_style: None,
        }
    }

    pub fn icon_size(mut self, icon_size: IconSize) -> Self {
        self.icon_size = icon_size;
        self
    }

    pub fn icon_color(mut self, icon_color: Color) -> Self {
        self.icon_color = icon_color;
        self
    }

    pub fn selected_icon(mut self, icon: impl Into<Option<Icon>>) -> Self {
        self.selected_icon = icon.into();
        self
    }
}

impl Disableable for IconButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.base = self.base.disabled(disabled);
        self
    }
}

impl Selectable for IconButton {
    fn selected(mut self, selected: bool) -> Self {
        self.base = self.base.selected(selected);
        self
    }
}

impl Clickable for IconButton {
    fn on_click(
        mut self,
        handler: impl Fn(&gpui::ClickEvent, &mut WindowContext) + 'static,
    ) -> Self {
        self.base = self.base.on_click(handler);
        self
    }
}

impl FixedWidth for IconButton {
    fn width(mut self, width: DefiniteLength) -> Self {
        self.base = self.base.width(width);
        self
    }

    fn full_width(mut self) -> Self {
        self.base = self.base.full_width();
        self
    }
}

impl ButtonCommon for IconButton {
    fn id(&self) -> &ElementId {
        self.base.id()
    }

    fn style(mut self, style: ButtonStyle) -> Self {
        self.base = self.base.style(style);
        self
    }

    fn size(mut self, size: ButtonSize) -> Self {
        self.base = self.base.size(size);
        self
    }

    fn tooltip(mut self, tooltip: impl Fn(&mut WindowContext) -> AnyView + 'static) -> Self {
        self.base = self.base.tooltip(tooltip);
        self
    }

    fn selected_style(mut self, style: impl Into<Option<ButtonStyle>>) -> Self {
        self.selected_style = style.into();
        self
    }
}

impl RenderOnce for IconButton {
    type Rendered = ButtonLike;

    fn render(self, _cx: &mut WindowContext) -> Self::Rendered {
        let is_disabled = self.base.disabled;
        let is_selected = self.base.selected;

        self.base.child(
            ButtonIcon::new(self.icon)
                .disabled(is_disabled)
                .selected(is_selected)
                .selected_icon(self.selected_icon)
                .size(self.icon_size)
                .color(self.icon_color),
        )
    }
}
