use std::{any::TypeId, sync::Arc};

use crate::{
    DispatchPhase, FocusEvent, FocusHandle, Interactive, KeyDownEvent, KeyUpEvent, StyleRefinement,
    ViewContext,
};

pub trait Focus: Interactive {
    fn set_focus_style(&mut self, style: StyleRefinement);
    fn set_focus_in_style(&mut self, style: StyleRefinement);
    fn set_in_focus_style(&mut self, style: StyleRefinement);
    fn handle(&self) -> &FocusHandle;

    fn focus(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self
    where
        Self: Sized,
    {
        self.set_focus_style(f(StyleRefinement::default()));
        self
    }

    fn focus_in(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self
    where
        Self: Sized,
    {
        self.set_focus_in_style(f(StyleRefinement::default()));
        self
    }

    fn in_focus(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self
    where
        Self: Sized,
    {
        self.set_in_focus_style(f(StyleRefinement::default()));
        self
    }

    fn on_focus(
        mut self,
        listener: impl Fn(&mut Self::ViewState, &FocusEvent, &mut ViewContext<Self::ViewState>)
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        let handle = self.handle().clone();
        self.listeners()
            .focus
            .push(Arc::new(move |view, event, cx| {
                if event.focused.as_ref() == Some(&handle) {
                    listener(view, event, cx)
                }
            }));
        self
    }

    fn on_blur(
        mut self,
        listener: impl Fn(&mut Self::ViewState, &FocusEvent, &mut ViewContext<Self::ViewState>)
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        let handle = self.handle().clone();
        self.listeners()
            .focus
            .push(Arc::new(move |view, event, cx| {
                if event.blurred.as_ref() == Some(&handle) {
                    listener(view, event, cx)
                }
            }));
        self
    }

    fn on_focus_in(
        mut self,
        listener: impl Fn(&mut Self::ViewState, &FocusEvent, &mut ViewContext<Self::ViewState>)
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        let handle = self.handle().clone();
        self.listeners()
            .focus
            .push(Arc::new(move |view, event, cx| {
                let descendant_blurred = event
                    .blurred
                    .as_ref()
                    .map_or(false, |blurred| handle.contains(blurred, cx));
                let descendant_focused = event
                    .focused
                    .as_ref()
                    .map_or(false, |focused| handle.contains(focused, cx));

                if !descendant_blurred && descendant_focused {
                    listener(view, event, cx)
                }
            }));
        self
    }

    fn on_focus_out(
        mut self,
        listener: impl Fn(&mut Self::ViewState, &FocusEvent, &mut ViewContext<Self::ViewState>)
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        let handle = self.handle().clone();
        self.listeners()
            .focus
            .push(Arc::new(move |view, event, cx| {
                let descendant_blurred = event
                    .blurred
                    .as_ref()
                    .map_or(false, |blurred| handle.contains(blurred, cx));
                let descendant_focused = event
                    .focused
                    .as_ref()
                    .map_or(false, |focused| handle.contains(focused, cx));
                if descendant_blurred && !descendant_focused {
                    listener(view, event, cx)
                }
            }));
        self
    }

    fn on_key_down(
        mut self,
        listener: impl Fn(
                &mut Self::ViewState,
                &KeyDownEvent,
                DispatchPhase,
                &mut ViewContext<Self::ViewState>,
            ) + Send
            + Sync
            + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        self.listeners().key.push((
            TypeId::of::<KeyDownEvent>(),
            Arc::new(move |view, event, phase, cx| {
                let event = event.downcast_ref().unwrap();
                listener(view, event, phase, cx)
            }),
        ));
        self
    }

    fn on_key_up(
        mut self,
        listener: impl Fn(&mut Self::ViewState, &KeyUpEvent, DispatchPhase, &mut ViewContext<Self::ViewState>)
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        self.listeners().key.push((
            TypeId::of::<KeyUpEvent>(),
            Arc::new(move |view, event, phase, cx| {
                let event = event.downcast_ref().unwrap();
                listener(view, event, phase, cx)
            }),
        ));
        self
    }
}