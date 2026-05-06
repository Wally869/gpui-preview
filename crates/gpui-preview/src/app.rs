use std::collections::HashMap;

use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    ActiveTheme as _, IconName, Side, Theme, ThemeMode,
    color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState},
    h_flex,
    input::{Input, InputEvent, InputState},
    resizable::{h_resizable, resizable_panel},
    scroll::ScrollableElement as _,
    sidebar::{Sidebar, SidebarGroup, SidebarMenu, SidebarMenuItem},
    slider::{Slider, SliderEvent, SliderState, SliderValue},
    switch::Switch,
    v_flex,
};

use crate::registry::{AnyPreviewable, PreviewEntry};
use crate::stories::{self, AllStories};
use crate::types::*;

actions!(preview, [SelectPrev, SelectNext, CloseDialog]);

// ── PreviewPanel (canvas + prop editor + stories) ────────────────────

struct PreviewPanel {
    entry: Option<&'static PreviewEntry>,
    current_instance: Option<Box<dyn AnyPreviewable>>,
    current_fields: Vec<FieldMeta>,
    text_inputs: HashMap<&'static str, Entity<InputState>>,
    slider_states: HashMap<&'static str, Entity<SliderState>>,
    color_picker_states: HashMap<&'static str, Entity<ColorPickerState>>,

    // Stories
    all_stories: AllStories,
    active_story: Option<String>,
    story_name_input: Entity<InputState>,
    show_save_dialog: bool,

    _subscriptions: Vec<Subscription>,
}

impl PreviewPanel {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let story_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Story name..."));

        Self {
            entry: None,
            current_instance: None,
            current_fields: Vec::new(),
            text_inputs: HashMap::new(),
            slider_states: HashMap::new(),
            color_picker_states: HashMap::new(),
            all_stories: stories::load_stories(),
            active_story: None,
            story_name_input,
            show_save_dialog: false,
            _subscriptions: Vec::new(),
        }
    }

    fn load_entry(
        &mut self,
        entry: &'static PreviewEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.entry = Some(entry);
        self.current_fields = (entry.fields)();
        self.current_instance = Some((entry.create_default)());
        self.active_story = None;
        self.show_save_dialog = false;

        self.rebuild_controls(window, cx);
        cx.notify();
    }

    fn rebuild_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.text_inputs.clear();
        self.slider_states.clear();
        self.color_picker_states.clear();
        self._subscriptions.clear();

        let fields: Vec<_> = self
            .current_fields
            .iter()
            .map(|f| (f.name, f.control.clone()))
            .collect();
        for (name, control) in &fields {
            self.build_control_for(name, control, window, cx);
        }
    }

    fn build_control_for(
        &mut self,
        name: &'static str,
        control: &ControlKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match control {
            ControlKind::TextInput => {
                let initial = self.current_string(name);
                let input = cx.new(|cx| InputState::new(window, cx).default_value(initial));
                self._subscriptions
                    .push(cx.subscribe(&input, move |this, state, event, cx| {
                        if matches!(event, InputEvent::Change) {
                            let val = state.read(cx).value().to_string();
                            this.update_field(name, FieldValue::String(val));
                            cx.notify();
                        }
                    }));
                self.text_inputs.insert(name, input);
            }
            ControlKind::NumberSlider { min, max } => {
                let initial = self.current_float(name) as f32;
                let slider = cx.new(|_| {
                    SliderState::new()
                        .min(*min as f32)
                        .max(*max as f32)
                        .default_value(initial)
                });
                self._subscriptions
                    .push(cx.subscribe(&slider, move |this, _, event, cx| {
                        let SliderEvent::Change(value) = event;
                        let f = match value {
                            SliderValue::Single(v) => *v as f64,
                            SliderValue::Range(a, _) => *a as f64,
                        };
                        this.update_field(name, FieldValue::Float(f));
                        cx.notify();
                    }));
                self.slider_states.insert(name, slider);
            }
            ControlKind::Color => {
                let initial_color = self.current_color(name);
                let picker = cx.new(|cx| {
                    let mut state = ColorPickerState::new(window, cx);
                    if let Some(hsla) = initial_color {
                        state = state.default_value(hsla);
                    }
                    state
                });
                self._subscriptions
                    .push(cx.subscribe(&picker, move |this, _, event, cx| {
                        let ColorPickerEvent::Change(color) = event;
                        match color {
                            Some(hsla) => {
                                let rgba: gpui::Rgba = (*hsla).into();
                                this.update_field(
                                    name,
                                    FieldValue::Color([
                                        (rgba.r * 255.0) as u8,
                                        (rgba.g * 255.0) as u8,
                                        (rgba.b * 255.0) as u8,
                                        (rgba.a * 255.0) as u8,
                                    ]),
                                );
                            }
                            None => {
                                this.update_field(name, FieldValue::Color([0, 0, 0, 255]));
                            }
                        }
                        cx.notify();
                    }));
                self.color_picker_states.insert(name, picker);
            }
            ControlKind::Optional(inner) => {
                self.build_control_for(name, inner, window, cx);
            }
            _ => {}
        }
    }

    // ── Story operations ─────────────────────────────────────────────

    fn apply_story(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.entry else { return };
        let component_id = entry.type_id().to_string();

        let story_fields = self
            .all_stories
            .get(&component_id)
            .and_then(|m| m.get(name))
            .cloned();

        let Some(story_fields) = story_fields else {
            return;
        };

        // Start from default instance
        let mut instance: Box<dyn AnyPreviewable> = (entry.create_default)();

        // Apply each story field
        for field in &self.current_fields {
            if let Some(json_val) = story_fields.get(field.name)
                && let Some(fv) = stories::json_to_field_value(json_val, &field.control)
            {
                instance = instance.set_field_boxed(field.name, fv);
            }
        }

        self.current_instance = Some(instance);
        self.active_story = Some(name.to_string());
        self.show_save_dialog = false;

        self.rebuild_controls(window, cx);
        cx.notify();
    }

    fn save_current_as_story(&mut self, name: String) {
        let Some(entry) = self.entry else { return };
        let Some(instance) = &self.current_instance else {
            return;
        };

        let fields_snapshot = stories::snapshot_fields(instance.as_ref(), &self.current_fields);
        let component_id = entry.type_id().to_string();

        self.all_stories
            .entry(component_id)
            .or_default()
            .insert(name.clone(), fields_snapshot);

        stories::save_stories(&self.all_stories);
        self.active_story = Some(name);
        self.show_save_dialog = false;
    }

    fn update_active_story(&mut self) {
        let Some(name) = self.active_story.clone() else {
            return;
        };
        self.save_current_as_story(name);
    }

    fn delete_active_story(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.entry else { return };
        let Some(name) = self.active_story.take() else {
            return;
        };
        let component_id = entry.type_id().to_string();

        if let Some(component_stories) = self.all_stories.get_mut(&component_id) {
            component_stories.remove(&name);
            if component_stories.is_empty() {
                self.all_stories.remove(&component_id);
            }
        }

        stories::save_stories(&self.all_stories);

        // Reset to default
        self.current_instance = Some((entry.create_default)());
        self.rebuild_controls(window, cx);
        cx.notify();
    }

    fn story_names(&self) -> Vec<String> {
        let Some(entry) = self.entry else {
            return Vec::new();
        };
        self.all_stories
            .get(entry.type_id())
            .map(|m| {
                let mut names: Vec<String> = m.keys().cloned().collect();
                names.sort();
                names
            })
            .unwrap_or_default()
    }

    // ── Field helpers ────────────────────────────────────────────────

    fn update_field(&mut self, name: &str, value: FieldValue) {
        if let Some(instance) = self.current_instance.take() {
            self.current_instance = Some(instance.set_field_boxed(name, value));
        }
    }

    fn current_value(&self, name: &str) -> Option<FieldValue> {
        self.current_instance.as_ref()?.get_field(name)
    }

    fn current_string(&self, name: &str) -> String {
        match self.current_value(name) {
            Some(FieldValue::String(s)) => s,
            _ => String::new(),
        }
    }

    fn current_float(&self, name: &str) -> f64 {
        match self.current_value(name) {
            Some(FieldValue::Float(f)) => f,
            Some(FieldValue::Int(i)) => i as f64,
            _ => 0.0,
        }
    }

    fn current_bool(&self, name: &str) -> bool {
        matches!(self.current_value(name), Some(FieldValue::Bool(true)))
    }

    fn current_enum(&self, name: &str) -> String {
        match self.current_value(name) {
            Some(FieldValue::Enum(s)) => s,
            _ => String::new(),
        }
    }

    fn current_color(&self, name: &str) -> Option<Hsla> {
        match self.current_value(name) {
            Some(FieldValue::Color(c)) => Some(Hsla::from(Rgba {
                r: c[0] as f32 / 255.0,
                g: c[1] as f32 / 255.0,
                b: c[2] as f32 / 255.0,
                a: c[3] as f32 / 255.0,
            })),
            _ => None,
        }
    }

    fn is_optional_some(&self, name: &str) -> bool {
        !matches!(self.current_value(name), Some(FieldValue::None) | None)
    }

    fn reset_to_default(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.entry else { return };
        self.current_instance = Some((entry.create_default)());
        self.active_story = None;
        self.rebuild_controls(window, cx);
        cx.notify();
    }

    fn set_optional_to_default(&mut self, name: &str) {
        let Some(field) = self.current_fields.iter().find(|f| f.name == name) else {
            return;
        };
        let ControlKind::Optional(inner) = &field.control else {
            return;
        };
        let default_value = match inner.as_ref() {
            ControlKind::TextInput => FieldValue::String(String::new()),
            ControlKind::Toggle => FieldValue::Bool(false),
            ControlKind::NumberSlider { min, .. } => FieldValue::Float(*min),
            ControlKind::Color => FieldValue::Color([0, 0, 0, 255]),
            _ => FieldValue::String(String::new()),
        };
        self.update_field(name, default_value);
    }

    fn render_inner_control(
        &self,
        name: &'static str,
        inner: &ControlKind,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match inner {
            ControlKind::TextInput => {
                if let Some(input) = self.text_inputs.get(name) {
                    Input::new(input).into_any_element()
                } else {
                    div().child("—").into_any_element()
                }
            }
            ControlKind::Toggle => {
                let checked = self.current_bool(name);
                Switch::new(SharedString::from(format!("{}-inner", name)))
                    .checked(checked)
                    .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                        this.update_field(name, FieldValue::Bool(*checked));
                        cx.notify();
                    }))
                    .into_any_element()
            }
            ControlKind::NumberSlider { .. } => {
                if let Some(slider) = self.slider_states.get(name) {
                    let val = format!("{:.1}", self.current_float(name));
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().flex_1().child(Slider::new(slider)))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .min_w(px(36.))
                                .child(val),
                        )
                        .into_any_element()
                } else {
                    div().child("—").into_any_element()
                }
            }
            ControlKind::Color => {
                if let Some(picker) = self.color_picker_states.get(name) {
                    ColorPicker::new(picker).into_any_element()
                } else {
                    div().child("—").into_any_element()
                }
            }
            _ => div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Unsupported")
                .into_any_element(),
        }
    }
}

impl Render for PreviewPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_name = self.entry.map_or("", |e| e.name);
        let active_desc = self.entry.map_or("", |e| e.description);

        // Render the component preview
        let preview_element: AnyElement = match (&self.current_instance, self.entry) {
            (Some(instance), Some(entry)) => (entry.render)(instance.as_any()),
            _ => div()
                .text_color(cx.theme().muted_foreground)
                .text_sm()
                .child("Select a component")
                .into_any_element(),
        };

        // === STORIES BAR ===
        let story_names = self.story_names();
        let active_story = self.active_story.clone();
        let has_active = active_story.is_some();

        let stories_bar = h_flex()
            .gap_2()
            .items_center()
            .flex_wrap()
            .p_4()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().muted_foreground)
                    .child("Stories"),
            )
            // Story chips
            .children(story_names.iter().map(|name| {
                let is_active = active_story.as_deref() == Some(name.as_str());
                let name_owned = name.clone();
                div()
                    .id(SharedString::from(format!("story-{}", name)))
                    .px_2()
                    .py(px(2.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .text_xs()
                    .when(is_active, |el| {
                        el.bg(cx.theme().primary)
                            .text_color(cx.theme().primary_foreground)
                    })
                    .when(!is_active, |el| {
                        el.bg(cx.theme().muted).hover(|el| el.bg(cx.theme().accent))
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.apply_story(&name_owned, window, cx);
                    }))
                    .child(name.clone())
            }))
            // "Custom" chip (deselects story, keeps current state)
            .when(!story_names.is_empty(), |el| {
                let is_custom = !has_active;
                el.child(
                    div()
                        .id("story-custom")
                        .px_2()
                        .py(px(2.))
                        .rounded(px(4.))
                        .cursor_pointer()
                        .text_xs()
                        .when(is_custom, |el| {
                            el.bg(cx.theme().primary)
                                .text_color(cx.theme().primary_foreground)
                        })
                        .when(!is_custom, |el| {
                            el.bg(cx.theme().muted).hover(|el| el.bg(cx.theme().accent))
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.active_story = None;
                            cx.notify();
                        }))
                        .child("Custom"),
                )
            })
            // Separator
            .child(div().w(px(1.)).h(px(16.)).bg(cx.theme().border))
            // Action buttons
            .child(
                div()
                    .id("story-new")
                    .px_2()
                    .py(px(2.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .hover(|el| el.bg(cx.theme().muted))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_save_dialog = !this.show_save_dialog;
                        cx.notify();
                    }))
                    .child("+ New"),
            )
            .when(has_active, |el| {
                el.child(
                    div()
                        .id("story-save")
                        .px_2()
                        .py(px(2.))
                        .rounded(px(4.))
                        .cursor_pointer()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .hover(|el| el.bg(cx.theme().muted))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.update_active_story();
                            cx.notify();
                        }))
                        .child("Save"),
                )
                .child(
                    div()
                        .id("story-delete")
                        .px_2()
                        .py(px(2.))
                        .rounded(px(4.))
                        .cursor_pointer()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .hover(|el| el.bg(cx.theme().danger.opacity(0.1)))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.delete_active_story(window, cx);
                        }))
                        .child("Delete"),
                )
            });

        // Save dialog (inline input)
        let save_dialog = if self.show_save_dialog {
            Some(
                h_flex()
                    .gap_2()
                    .items_center()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().flex_1().child(Input::new(&self.story_name_input)))
                    .child(
                        div()
                            .id("story-confirm-save")
                            .px_3()
                            .py_1()
                            .rounded(px(4.))
                            .cursor_pointer()
                            .text_xs()
                            .bg(cx.theme().primary)
                            .text_color(cx.theme().primary_foreground)
                            .hover(|el| el.opacity(0.8))
                            .on_click(cx.listener(|this, _, window, cx| {
                                let name =
                                    this.story_name_input.read(cx).value().trim().to_string();
                                if !name.is_empty() {
                                    this.save_current_as_story(name);
                                    this.story_name_input.update(cx, |state, cx| {
                                        state.set_value("", window, cx);
                                    });
                                    cx.notify();
                                }
                            }))
                            .child("Save"),
                    )
                    .child(
                        div()
                            .id("story-cancel-save")
                            .px_3()
                            .py_1()
                            .rounded(px(4.))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .hover(|el| el.bg(cx.theme().muted))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_save_dialog = false;
                                cx.notify();
                            }))
                            .child("Cancel"),
                    ),
            )
        } else {
            None
        };

        // Build prop controls
        let fields = self.current_fields.clone();
        let prop_controls: Vec<AnyElement> = fields
            .iter()
            .map(|field| {
                let control: AnyElement = match &field.control {
                    ControlKind::TextInput => {
                        if let Some(input) = self.text_inputs.get(field.name) {
                            Input::new(input).into_any_element()
                        } else {
                            div().child("—").into_any_element()
                        }
                    }
                    ControlKind::Toggle => {
                        let checked = self.current_bool(field.name);
                        let name = field.name;
                        Switch::new(field.name)
                            .checked(checked)
                            .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                this.update_field(name, FieldValue::Bool(*checked));
                                cx.notify();
                            }))
                            .into_any_element()
                    }
                    ControlKind::NumberSlider { .. } => {
                        if let Some(slider) = self.slider_states.get(field.name) {
                            let val = format!("{:.1}", self.current_float(field.name));
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(div().flex_1().child(Slider::new(slider)))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .min_w(px(36.))
                                        .child(val),
                                )
                                .into_any_element()
                        } else {
                            div().child("—").into_any_element()
                        }
                    }
                    ControlKind::Select(variants) => {
                        let current = self.current_enum(field.name);
                        let name = field.name;
                        let variants_owned: Vec<String> =
                            variants.iter().map(|v| v.to_string()).collect();

                        v_flex()
                            .gap_1()
                            .children(variants.iter().enumerate().map(|(i, variant)| {
                                let is_selected = *variant == current;
                                let variants_ref = variants_owned.clone();
                                div()
                                    .id(SharedString::from(format!("{}-{}", field.name, variant)))
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .text_sm()
                                    .when(is_selected, |el| {
                                        el.bg(cx.theme().primary)
                                            .text_color(cx.theme().primary_foreground)
                                    })
                                    .when(!is_selected, |el| el.hover(|el| el.bg(cx.theme().muted)))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(variant) = variants_ref.get(i) {
                                            this.update_field(
                                                name,
                                                FieldValue::Enum(variant.clone()),
                                            );
                                            cx.notify();
                                        }
                                    }))
                                    .child(*variant)
                            }))
                            .into_any_element()
                    }
                    ControlKind::Color => {
                        if let Some(picker) = self.color_picker_states.get(field.name) {
                            ColorPicker::new(picker).into_any_element()
                        } else {
                            div().child("—").into_any_element()
                        }
                    }
                    ControlKind::Optional(inner) => {
                        let is_some = self.is_optional_some(field.name);
                        let name = field.name;
                        let inner_control: AnyElement = if is_some {
                            self.render_inner_control(field.name, inner, cx)
                        } else {
                            div().into_any_element()
                        };
                        v_flex()
                            .gap_2()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        Switch::new(SharedString::from(format!(
                                            "{}-opt",
                                            field.name
                                        )))
                                        .checked(is_some)
                                        .on_click(
                                            cx.listener(move |this, checked: &bool, _, cx| {
                                                if *checked {
                                                    this.set_optional_to_default(name);
                                                } else {
                                                    this.update_field(name, FieldValue::None);
                                                }
                                                cx.notify();
                                            }),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(if is_some { "Some" } else { "None" }),
                                    ),
                            )
                            .when(is_some, |el| el.child(inner_control))
                            .into_any_element()
                    }
                    _ => div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Unsupported control")
                        .into_any_element(),
                };

                // Wrap in labeled container
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(field.name),
                    )
                    .when(!field.doc.is_empty(), |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(field.doc),
                        )
                    })
                    .child(control)
                    .into_any_element()
            })
            .collect();

        // === CANVAS ===
        let canvas = v_flex()
            .flex_1()
            .h_full()
            .child(
                v_flex()
                    .p_4()
                    .gap_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child(active_name),
                    )
                    .when(!active_desc.is_empty(), |el| {
                        el.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(active_desc),
                        )
                    }),
            )
            .when(self.entry.is_some(), |el| el.child(stories_bar))
            .when_some(save_dialog, |el, dialog| el.child(dialog))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .p_8()
                    .child(preview_element),
            );

        // === PROP EDITOR ===
        let prop_editor = v_flex()
            .h_full()
            .w(px(300.))
            .min_w(px(200.))
            .overflow_y_scrollbar()
            .border_l_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .p_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Properties"),
                            )
                            .when(self.entry.is_some(), |el| {
                                el.child(
                                    div()
                                        .id("reset-defaults")
                                        .px_2()
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .hover(|el| el.bg(cx.theme().muted))
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.reset_to_default(window, cx);
                                        }))
                                        .child("Reset"),
                                )
                            }),
                    ),
            )
            .child(v_flex().p_4().gap_4().children(prop_controls));

        h_flex().size_full().child(canvas).child(prop_editor)
    }
}

// ── PreviewApp (sidebar + layout) ────────────────────────────────────

/// The main preview application view.
pub struct PreviewApp {
    entries: Vec<&'static PreviewEntry>,
    active_index: Option<usize>,
    focus_handle: FocusHandle,
    search_input: Entity<InputState>,
    panel: Entity<PreviewPanel>,
    _subscriptions: Vec<Subscription>,
}

impl PreviewApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut entries: Vec<&'static PreviewEntry> = inventory::iter::<PreviewEntry>().collect();
        entries.sort_by_key(|e| (e.category, e.name));

        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search components..."));

        let search_sub = cx.subscribe(&search_input, |_this, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });

        let focus_handle = cx.focus_handle();
        let panel = cx.new(|cx| PreviewPanel::new(window, cx));

        let mut app = Self {
            entries,
            active_index: None,
            focus_handle,
            search_input,
            panel,
            _subscriptions: vec![search_sub],
        };

        if !app.entries.is_empty() {
            app.select_component(0, window, cx);
        }

        app
    }

    fn select_component(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.active_index = Some(index);
        let entry = self.entries[index];
        self.panel.update(cx, |panel, cx| {
            panel.load_entry(entry, window, cx);
        });
        cx.notify();
    }
}

impl Render for PreviewApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search_input.read(cx).value().trim().to_lowercase();

        // Group filtered entries by category
        let mut groups: Vec<(&str, Vec<(usize, &str)>)> = Vec::new();
        for (idx, entry) in self.entries.iter().enumerate() {
            if !query.is_empty() && !entry.name.to_lowercase().contains(&query) {
                continue;
            }
            match groups.last_mut() {
                Some((cat, items)) if *cat == entry.category => {
                    items.push((idx, entry.name));
                }
                _ => {
                    groups.push((entry.category, vec![(idx, entry.name)]));
                }
            }
        }

        // === SIDEBAR ===
        let is_dark = cx.theme().mode.is_dark();
        let theme_icon = if is_dark {
            IconName::Sun
        } else {
            IconName::Moon
        };

        let sidebar = Sidebar::new(Side::Left)
            .w(relative(1.))
            .border_0()
            .header(
                v_flex()
                    .w_full()
                    .gap_3()
                    .child(
                        h_flex().px_2().py_1().items_center().justify_end().child(
                            div()
                                .id("theme-toggle")
                                .cursor_pointer()
                                .p_1()
                                .rounded(px(4.))
                                .text_color(cx.theme().muted_foreground)
                                .hover(|el| el.bg(cx.theme().muted))
                                .on_click(cx.listener(move |_this, _, window, cx| {
                                    let new_mode = if is_dark {
                                        ThemeMode::Light
                                    } else {
                                        ThemeMode::Dark
                                    };
                                    Theme::change(new_mode, Some(window), cx);
                                    cx.notify();
                                }))
                                .child(theme_icon),
                        ),
                    )
                    .child(
                        div().mx_1().child(
                            Input::new(&self.search_input)
                                .appearance(false)
                                .cleanable(true),
                        ),
                    ),
            )
            .children(groups.into_iter().map(|(category, items)| {
                SidebarGroup::new(category).child(SidebarMenu::new().children(
                    items.into_iter().map(|(idx, name)| {
                        SidebarMenuItem::new(name)
                            .active(self.active_index == Some(idx))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.select_component(idx, window, cx);
                            }))
                    }),
                ))
            }));

        // === LAYOUT ===
        div()
            .id("preview-root")
            .size_full()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &SelectNext, window, cx| {
                let len = this.entries.len();
                if len == 0 {
                    return;
                }
                let next = match this.active_index {
                    Some(i) => (i + 1) % len,
                    None => 0,
                };
                this.select_component(next, window, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectPrev, window, cx| {
                let len = this.entries.len();
                if len == 0 {
                    return;
                }
                let prev = match this.active_index {
                    Some(i) => (i + len - 1) % len,
                    None => 0,
                };
                this.select_component(prev, window, cx);
            }))
            .on_action(cx.listener(|this, _: &CloseDialog, _, cx| {
                this.panel.update(cx, |panel, cx| {
                    if panel.show_save_dialog {
                        panel.show_save_dialog = false;
                        cx.notify();
                    }
                });
            }))
            .child(
                h_resizable("preview-layout")
                    .child(
                        resizable_panel()
                            .size(px(240.))
                            .size_range(px(180.)..px(350.))
                            .child(sidebar),
                    )
                    .child(self.panel.clone().into_any_element()),
            )
    }
}
