//! Display an interactive selector of a two sliders that cannot cross one another in a single range of values.
//!
//! A [`DoubleSlider`] has some local [`State`].
use iced::{touch, Theme};
use iced_core::border;
use iced_core::event::{self, Event};
use iced_core::keyboard;
use iced_core::layout;
use iced_core::mouse;
use iced_core::renderer;
use iced_core::widget::tree::{self, Tree};
use iced_core::Color;
use iced_core::{
    Border, Clipboard, Element, Layout, Length, Pixels, Point, Rectangle, Shell, Size, Widget,
};

use std::cmp::min;
use std::fmt::Debug;
use std::ops::RangeInclusive;

/// An horizontal bar and a handle that selects a single value from a range of
/// values.
///
/// A [`Slider`] will try to fill the horizontal space of its container.
///
/// The [`Slider`] range of numeric values is generic and its step size defaults
/// to 1 unit.
///
/// # Example
/// ```no_run
/// # type Slider<'a, T, Message> =
/// #     iced_widget::Slider<'a, Message, T, iced_widget::style::Theme>;
/// #
/// #[derive(Clone)]
/// pub enum Message {
///     SliderChanged(f32),
/// }
///
/// let value = 50.0;
///
/// Slider::new(0.0..=100.0, value, Message::SliderChanged);
/// ```
///
/// ![Slider drawn by Coffee's renderer](https://github.com/hecrj/coffee/blob/bda9818f823dfcb8a7ad0ff4940b4d4b387b5208/images/ui/slider.png?raw=true)
#[allow(missing_debug_implementations)]
pub struct DoubleSlider<'a, T, Message, Theme = iced::Theme>
where
    Theme: StyleSheet,
    T: Into<f64>,
{
    range: RangeInclusive<T>,
    step: T,
    shift_step: Option<T>,
    left_value: T,
    right_value: T,
    left_default: Option<T>,
    right_default: Option<T>,
    on_change: Box<dyn Fn((T, SliderSide)) -> Message + 'a>,
    on_release: Option<Message>,
    width: Length,
    height: f32,
    handle_width: f32,
    handle_height: f32,
    style: Theme::Style,
}

impl<'a, T, Message, Theme> DoubleSlider<'a, T, Message, Theme>
where
    T: Copy + From<u8> + Into<f64> + Ord,
    Message: Clone,
    Theme: StyleSheet,
{
    /// The default height of a [`Slider`].
    pub const DEFAULT_HEIGHT: f32 = 22.0;

    /// Creates a new [`Slider`].
    ///
    /// It expects:
    ///   * an inclusive range of possible values
    ///   * the current value of the [`Slider`]
    ///   * a function that will be called when the [`Slider`] is dragged.
    ///   It receives the new value of the [`Slider`] and must produce a
    ///   `Message`.
    pub fn new<F>(range: RangeInclusive<T>, left_value: T, right_value: T, on_change: F) -> Self
    where
        F: 'a + Fn((T, SliderSide)) -> Message,
    {
        let left_value = if left_value >= *range.start() {
            left_value
        } else {
            *range.start()
        };

        let left_value = if left_value <= *range.end() {
            left_value
        } else {
            *range.end()
        };

        let right_value = if right_value >= *range.start() {
            right_value
        } else {
            *range.start()
        };

        let right_value = if right_value <= *range.end() {
            right_value
        } else {
            *range.end()
        };

        // shouldn't ever have left > right
        let left_value = min(left_value, right_value);

        let style = Theme::Style::default();

        DoubleSlider {
            left_value,
            right_value,
            left_default: Some(*range.start()),
            right_default: Some(*range.end()),
            range,
            step: T::from(1),
            shift_step: None,
            on_change: Box::new(on_change),
            on_release: None,
            width: Length::Fill,
            height: Self::DEFAULT_HEIGHT,
            handle_width: 8.0,
            handle_height: 20.0,
            style,
        }
    }

    /// Sets the optional default value for the [`Slider`].
    ///
    /// If set, the [`Slider`] will reset to this value when ctrl-clicked or command-clicked.
    pub fn default(mut self, left_default: impl Into<T>, right_default: impl Into<T>) -> Self {
        self.left_default = Some(left_default.into());
        self.right_default = Some(right_default.into());
        self
    }

    /// Sets the release message of the [`Slider`].
    /// This is called when the mouse is released from the slider.
    ///
    /// Typically, the user's interaction with the slider is finished when this message is produced.
    /// This is useful if you need to spawn a long-running task from the slider's result, where
    /// the default on_change message could create too many events.
    pub fn on_release(mut self, on_release: Message) -> Self {
        self.on_release = Some(on_release);
        self
    }

    /// Sets the width of the [`Slider`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Slider`].
    pub fn height(mut self, height: impl Into<Pixels>) -> Self {
        self.height = height.into().0;
        self
    }

    /// Sets the style of the [`Slider`].
    pub fn style(mut self, style: impl Into<Theme::Style>) -> Self {
        self.style = style.into();
        self
    }

    /// Sets the step size of the [`Slider`].
    pub fn step(mut self, step: impl Into<T>) -> Self {
        self.step = step.into();
        self
    }

    /// Sets the optional "shift" step for the [`Slider`].
    ///
    /// If set, this value is used as the step while the shift key is pressed.
    pub fn shift_step(mut self, shift_step: impl Into<T>) -> Self {
        self.shift_step = Some(shift_step.into());
        self
    }
}

impl<'a, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for DoubleSlider<'a, T, Message, Theme>
where
    T: Copy + Into<f64> + num_traits::FromPrimitive + Debug,
    Message: Clone,
    Theme: StyleSheet,
    Renderer: iced_core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let dims = limits.resolve(self.width, self.height, Size::ZERO);
        // get the positions relative to the parent
        let start: f64 = (*self.range.start()).into();
        let end: f64 = (*self.range.end()).into();

        let left_x: f32 = ((self.left_value.into() - start) / (end - start)) as f32 * dims.width
            - self.handle_width / 2.0;
        let right_x: f32 = ((self.right_value.into() - start) / (end - start)) as f32 * dims.width
            - self.handle_width / 2.0;
        let left_slider = layout::Node::new(Size::new(self.handle_width, self.handle_height))
            .move_to(Point::new(left_x, 0.0));
        let right_slider = layout::Node::new(Size::new(self.handle_width, self.handle_height))
            .move_to(Point::new(right_x, 0.0));

        layout::Node::with_children(dims, vec![left_slider, right_slider])
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        update(
            event,
            layout,
            cursor,
            shell,
            tree.state.downcast_mut::<State>(),
            &mut self.left_value,
            &mut self.right_value,
            self.left_default,
            self.right_default,
            &self.range,
            self.step,
            self.shift_step,
            self.on_change.as_ref(),
            &self.on_release,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        draw(
            renderer,
            layout,
            cursor,
            tree.state.downcast_ref::<State>(),
            self.left_value,
            self.right_value,
            &self.range,
            theme,
            &self.style,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse_interaction(layout, cursor, tree.state.downcast_ref::<State>())
    }
}

impl<'a, T, Message, Theme, Renderer> From<DoubleSlider<'a, T, Message, Theme>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Copy + Into<f64> + num_traits::FromPrimitive + 'a + Debug,
    Message: Clone + 'a,
    Theme: StyleSheet + 'a,
    Renderer: iced_core::Renderer + 'a,
{
    fn from(slider: DoubleSlider<'a, T, Message, Theme>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(slider)
    }
}

/// Processes an [`Event`] and updates the [`State`] of a [`Slider`]
/// accordingly.
pub fn update<Message, T>(
    event: Event,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    shell: &mut Shell<'_, Message>,
    state: &mut State,
    left_value: &mut T,
    right_value: &mut T,
    left_default: Option<T>,
    right_default: Option<T>,
    range: &RangeInclusive<T>,
    step: T,
    shift_step: Option<T>,
    on_change: &dyn Fn((T, SliderSide)) -> Message,
    on_release: &Option<Message>,
) -> event::Status
where
    T: Copy + Into<f64> + Debug + num_traits::FromPrimitive,
    Message: Clone,
{
    let is_dragging = state.is_dragging;
    let slider_side = state.slider_side;
    let (current_left_value, current_right_value) = (*left_value, *right_value);
    let layouts = layout.children().collect::<Vec<Layout>>();
    let (left_layout, right_layout) = (layouts[0], layouts[1]);

    // extract the value of the slider based on the cursor position
    let locate = |cursor_position: Point| -> Option<T> {
        let bounds = layout.bounds();
        /*We have several cases:
        1. slider_side is left
            a. if the xpos is less than the left bound, then return *range.start()
            b. if the xpos is greater than the right cursor, then return the right cursor position
        2. slider_side is right
            a. if the xpos is greater than the right bound, then return *range.end()
            b. if the xpos is less than than the left cursor, then return the left cursor position
        */

        if slider_side.is_none() {
            return None;
        }
        let slider_side = slider_side.unwrap();
        let start: f64 = (*range.start()).into();
        let end: f64 = (*range.end()).into();

        // get the left and right x values of the current sliders
        let left_x: f32 =
            ((current_left_value.into() - start) / (end - start)) as f32 * bounds.width + bounds.x;
        let right_x: f32 =
            ((current_right_value.into() - start) / (end - start)) as f32 * bounds.width + bounds.x;
        let new_value = match slider_side {
            SliderSide::Left if cursor_position.x <= bounds.x => Some(*range.start()),
            SliderSide::Left if cursor_position.x >= right_x => Some(current_right_value),
            SliderSide::Right if cursor_position.x >= bounds.x + bounds.width => Some(*range.end()),
            SliderSide::Right if cursor_position.x <= left_x => Some(current_left_value),
            _ => {
                let step = if state.keyboard_modifiers.shift() {
                    shift_step.unwrap_or(step)
                } else {
                    step
                }
                .into();

                let percent = f64::from(cursor_position.x - bounds.x) / f64::from(bounds.width);

                let steps = (percent * (end - start) / step).round();
                let value = steps * step + start;
                T::from_f64(value)
            }
        };
        new_value
    };

    // TODO: these are used when we implement keyboard use
    let increment = |value: T| -> Option<T> {
        let step = if state.keyboard_modifiers.shift() {
            shift_step.unwrap_or(step)
        } else {
            step
        }
        .into();

        let steps = (value.into() / step).round();
        let new_value = step * (steps + 1.0);

        if new_value > (*range.end()).into() {
            return Some(*range.end());
        }

        T::from_f64(new_value)
    };

    let decrement = |value: T| -> Option<T> {
        let step = if state.keyboard_modifiers.shift() {
            shift_step.unwrap_or(step)
        } else {
            step
        }
        .into();

        let steps = (value.into() / step).round();
        let new_value = step * (steps - 1.0);

        if new_value < (*range.start()).into() {
            return Some(*range.start());
        }

        T::from_f64(new_value)
    };

    let mut change = |new_value: Option<T>, side: SliderSide| match new_value {
        Some(new_value) => match side {
            SliderSide::Left if ((*left_value).into() - new_value.into()).abs() > f64::EPSILON => {
                shell.publish((on_change)((new_value, SliderSide::Left)));
                *left_value = new_value;
            }
            SliderSide::Right
                if ((*right_value).into() - new_value.into()).abs() > f64::EPSILON =>
            {
                shell.publish((on_change)((new_value, SliderSide::Right)));
                *right_value = new_value;
            }
            _ => (),
        },
        None => (),
    };

    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerPressed { .. }) => {
            if let Some(cursor_position) = cursor.position_over(left_layout.bounds()) {
                if state.keyboard_modifiers.command() {
                    change(left_default, SliderSide::Left);
                    state.is_dragging = false;
                } else {
                    change(locate(cursor_position), SliderSide::Left);
                    state.is_dragging = true;
                    state.slider_side = Some(SliderSide::Left);
                }

                return event::Status::Captured;
            } else if let Some(cursor_position) = cursor.position_over(right_layout.bounds()) {
                if state.keyboard_modifiers.command() {
                    change(right_default, SliderSide::Right);
                    state.is_dragging = false;
                } else {
                    change(locate(cursor_position), SliderSide::Right);
                    state.is_dragging = true;
                    state.slider_side = Some(SliderSide::Right);
                }

                return event::Status::Captured;
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerLifted { .. })
        | Event::Touch(touch::Event::FingerLost { .. }) => {
            if is_dragging {
                if let Some(on_release) = on_release.clone() {
                    shell.publish(on_release);
                }
                state.is_dragging = false;

                return event::Status::Captured;
            }
        }
        Event::Mouse(mouse::Event::CursorMoved { .. })
        | Event::Touch(touch::Event::FingerMoved { .. }) => {
            if is_dragging {
                match state.slider_side {
                    Some(side) => {
                        change(cursor.position().and_then(locate), side);
                    }
                    None => (),
                }

                return event::Status::Captured;
            }
        }
        // TODO(jhpick): implement increment/decrement for keyboard
        //Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
        //    if cursor.position_over(layout.bounds()).is_some() {
        //        match key {
        //            Key::Named(key::Named::ArrowUp) => {
        //                let _ = increment(current_value).map(change);
        //            }
        //            Key::Named(key::Named::ArrowDown) => {
        //                let _ = decrement(current_value).map(change);
        //            }
        //            _ => (),
        //        }

        //        return event::Status::Captured;
        //    }
        //}
        //Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
        //    state.keyboard_modifiers = modifiers;
        //}
        _ => {}
    }

    event::Status::Ignored
}

/// Draws a [`Slider`].
pub fn draw<T, Theme, Renderer>(
    renderer: &mut Renderer,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    state: &State,
    left_value: T,
    right_value: T,
    range: &RangeInclusive<T>,
    theme: &Theme,
    style: &Theme::Style,
) where
    T: Into<f64> + Copy,
    Theme: StyleSheet,
    Renderer: iced_core::Renderer,
{
    let bounds = layout.bounds();
    let is_mouse_over = cursor.is_over(bounds);

    let style = if state.is_dragging {
        theme.dragging(style)
    } else if is_mouse_over {
        theme.hovered(style)
    } else {
        theme.active(style)
    };

    let (handle_width, handle_height, handle_border_radius) = match style.handle.shape {
        HandleShape::Circle { radius } => (radius * 2.0, radius * 2.0, radius.into()),
        HandleShape::Rectangle {
            width,
            border_radius,
        } => (f32::from(width), bounds.height, border_radius),
    };

    let left_value = left_value.into() as f32;
    let right_value = right_value.into() as f32;
    let (range_start, range_end) = {
        let (start, end) = range.clone().into_inner();

        (start.into() as f32, end.into() as f32)
    };

    let left_offset = if range_start >= range_end {
        0.0
    } else {
        bounds.width * (left_value - range_start) / (range_end - range_start)
    };

    let right_offset = if range_start >= range_end {
        0.0
    } else {
        bounds.width * (right_value - range_start) / (range_end - range_start)
    };

    let rail_y = bounds.y + bounds.height / 2.0;

    // Left of the left handle
    renderer.fill_quad(
        renderer::Quad {
            bounds: Rectangle {
                x: bounds.x,
                y: rail_y - style.rail.width / 2.0,
                width: left_offset - handle_width / 2.0,
                height: style.rail.width,
            },
            border: Border::with_radius(style.rail.border_radius),
            ..renderer::Quad::default()
        },
        style.rail.colors.1,
    );

    //TODO(jhpick): remove these when done developing
    // left and right hitboxes
    let children = layout.children().collect::<Vec<Layout>>();
    let left_node = children[0];
    let right_node = children[1];
    renderer.fill_quad(
        renderer::Quad {
            bounds: Rectangle {
                x: left_node.position().x,
                y: left_node.position().y,
                width: left_node.bounds().width,
                height: left_node.bounds().height,
            },
            border: Border::default(),
            ..renderer::Quad::default()
        },
        Color::from_rgb8(0, 255, 0),
    );
    renderer.fill_quad(
        renderer::Quad {
            bounds: Rectangle {
                x: right_node.position().x,
                y: right_node.position().y,
                width: right_node.bounds().width,
                height: right_node.bounds().height,
            },
            border: Border::default(),
            ..renderer::Quad::default()
        },
        Color::from_rgb8(0, 0, 255),
    );

    // The left handle
    renderer.fill_quad(
        renderer::Quad {
            bounds: Rectangle {
                x: bounds.x + left_offset - handle_width / 2.0,
                y: rail_y - handle_height / 2.0,
                width: handle_width,
                height: handle_height,
            },
            border: Border {
                radius: handle_border_radius,
                width: style.handle.border_width,
                color: style.handle.border_color,
            },
            ..renderer::Quad::default()
        },
        style.handle.color,
    );

    // between the handles
    renderer.fill_quad(
        renderer::Quad {
            bounds: Rectangle {
                x: bounds.x + left_offset + handle_width / 2.0,
                y: rail_y - style.rail.width / 2.0,
                width: right_offset - left_offset - handle_width,
                height: style.rail.width,
            },
            border: Border::with_radius(style.rail.border_radius),
            ..renderer::Quad::default()
        },
        style.rail.colors.0,
    );

    // The right handle
    renderer.fill_quad(
        renderer::Quad {
            bounds: Rectangle {
                x: bounds.x + right_offset - handle_width / 2.0,
                y: rail_y - handle_height / 2.0,
                width: handle_width,
                height: handle_height,
            },
            border: Border {
                radius: handle_border_radius,
                width: style.handle.border_width,
                color: style.handle.border_color,
            },
            ..renderer::Quad::default()
        },
        style.handle.color,
    );

    // Right of the right handle
    renderer.fill_quad(
        renderer::Quad {
            bounds: Rectangle {
                x: bounds.x + right_offset + handle_width / 2.0,
                y: rail_y - style.rail.width / 2.0,
                width: bounds.width - right_offset,
                height: style.rail.width,
            },
            border: Border::with_radius(style.rail.border_radius),
            ..renderer::Quad::default()
        },
        style.rail.colors.1,
    );
}

/// Computes the current [`mouse::Interaction`] of a [`Slider`].
pub fn mouse_interaction(
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    state: &State,
) -> mouse::Interaction {
    let bounds = layout.bounds();
    let is_mouse_over = cursor.is_over(bounds);

    if state.is_dragging {
        mouse::Interaction::Grabbing
    } else if is_mouse_over {
        mouse::Interaction::Grab
    } else {
        mouse::Interaction::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderSide {
    Left,
    Right,
}

/// The local state of a [`Slider`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    is_dragging: bool,
    slider_side: Option<SliderSide>,
    keyboard_modifiers: keyboard::Modifiers,
}

impl State {
    /// Creates a new [`State`].
    pub fn new() -> State {
        State::default()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Appearance {
    /// The colors of the rail of the slider.
    pub rail: Rail,
    /// The appearance of the [`Handle`] of the slider.
    pub handle: Handle,
}

/// The appearance of a slider rail
#[derive(Debug, Clone, Copy)]
pub struct Rail {
    /// The colors of the rail of the slider.
    pub colors: (Color, Color),
    /// The width of the stroke of a slider rail.
    pub width: f32,
    /// The border radius of the corners of the rail.
    pub border_radius: border::Radius,
}

/// The appearance of the handle of a slider.
#[derive(Debug, Clone, Copy)]
pub struct Handle {
    /// The shape of the handle.
    pub shape: HandleShape,
    /// The [`Color`] of the handle.
    pub color: Color,
    /// The border width of the handle.
    pub border_width: f32,
    /// The border [`Color`] of the handle.
    pub border_color: Color,
}

/// The shape of the handle of a slider.
#[derive(Debug, Clone, Copy)]
pub enum HandleShape {
    /// A circular handle.
    Circle {
        /// The radius of the circle.
        radius: f32,
    },
    /// A rectangular shape.
    Rectangle {
        /// The width of the rectangle.
        width: u16,
        /// The border radius of the corners of the rectangle.
        border_radius: border::Radius,
    },
}

/// A set of rules that dictate the style of a slider.
pub trait StyleSheet {
    /// The supported style of the [`StyleSheet`].
    type Style: Default;

    /// Produces the style of an active slider.
    fn active(&self, style: &Self::Style) -> Appearance;

    /// Produces the style of an hovered slider.
    fn hovered(&self, style: &Self::Style) -> Appearance;

    /// Produces the style of a slider that is being dragged.
    fn dragging(&self, style: &Self::Style) -> Appearance;
}

/// The appearance of a slider.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub rail_colors: (Color, Color),
    pub handle: Handle,
}

/// The style of a slider.
#[derive(Default)]
pub enum DoubleSliderStyle {
    /// The default style.
    #[default]
    Default,
    /// A custom style.
    Custom(Box<dyn StyleSheet<Style = Theme>>),
}

impl StyleSheet for Theme {
    type Style = DoubleSliderStyle;

    fn active(&self, style: &Self::Style) -> Appearance {
        match style {
            DoubleSliderStyle::Default => {
                let palette = self.extended_palette();

                let handle = Handle {
                    shape: HandleShape::Rectangle {
                        width: 8,
                        border_radius: 4.0.into(),
                    },
                    color: Color::WHITE,
                    border_color: Color::WHITE,
                    border_width: 1.0,
                };

                Appearance {
                    rail: Rail {
                        colors: (palette.primary.base.color, palette.secondary.base.color),
                        width: 4.0,
                        border_radius: 2.0.into(),
                    },
                    handle: Handle {
                        color: palette.background.base.color,
                        border_color: palette.primary.base.color,
                        ..handle
                    },
                }
            }
            DoubleSliderStyle::Custom(custom) => custom.active(self),
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        match style {
            DoubleSliderStyle::Default => {
                let active = self.active(style);
                let palette = self.extended_palette();

                Appearance {
                    handle: Handle {
                        color: palette.primary.weak.color,
                        ..active.handle
                    },
                    ..active
                }
            }
            DoubleSliderStyle::Custom(custom) => custom.hovered(self),
        }
    }

    fn dragging(&self, style: &Self::Style) -> Appearance {
        match style {
            DoubleSliderStyle::Default => {
                let active = self.active(style);
                let palette = self.extended_palette();

                Appearance {
                    handle: Handle {
                        color: palette.primary.base.color,
                        ..active.handle
                    },
                    ..active
                }
            }
            DoubleSliderStyle::Custom(custom) => custom.dragging(self),
        }
    }
}
