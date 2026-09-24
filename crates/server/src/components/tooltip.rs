use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A short hint shown when its trigger is hovered or focused.
///
/// Pass the trigger and `tooltip_content` as children. Positioning does not adjust to
/// the viewport edges, so keep the hint short and leave room for it.
///
/// Give the trigger its own text or accessible label. The tooltip must not be the only
/// way to learn what the trigger does. To associate the hint with the trigger, give
/// `tooltip_content` an `id` and reference it with the trigger's `aria-describedby`.
///
/// ```ignore
/// view! {
///     tooltip(
///         button(
///             size: ButtonSize::Icon,
///             variant: ButtonVariant::Outline,
///             icon(data: iconify_icon!("lucide:copy"), label: "Copy link")
///         )
///         tooltip_content("Copy link")
///     )
/// }
/// ```
#[component]
pub async fn tooltip(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!("group relative inline-flex", attrs.remove("class")))
            (attrs)
        >
            (child)
        </span>
    })
}

/// Classes for a tooltip above its trigger. The bubble ignores pointer events. Opacity
/// and visibility transitions let it fade in and out.
const BUBBLE: StaticClass = class!(
    "pointer-events-none invisible absolute left-1/2 z-50 \
     -translate-x-1/2 rounded-md bg-foreground px-2.5 py-1 text-xs font-medium text-background \
     opacity-0 shadow-sm whitespace-nowrap \
     [transition:opacity_150ms_ease-out,visibility_150ms_allow-discrete] \
     group-hover:visible group-hover:opacity-100 \
     group-focus-within:visible group-focus-within:opacity-100",
);

/// The side where the tooltip bubble opens relative to its trigger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipSide {
    /// Open above the trigger.
    #[default]
    Top,
    /// Open below the trigger.
    Bottom,
}

impl TooltipSide {
    fn classes(self) -> StaticClass {
        match self {
            Self::Top => class!("bottom-full mb-2"),
            Self::Bottom => class!("top-full mt-2"),
        }
    }
}

/// The hint a [`tooltip`] shows beside its trigger.
#[component]
pub async fn tooltip_content(
    /// The side where the bubble opens. Defaults to [`TooltipSide::Top`].
    #[default]
    side: TooltipSide,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            role="tooltip"
            class=(class!(BUBBLE, side.classes(), attrs.remove("class")))
            (attrs)
        >
            (child)
        </span>
    })
}
