use leptos::prelude::*;
use leptos_ui::variants;

variants! {
    Badge {
        base: "inline-flex w-fit items-center rounded-md border font-semibold transition-colors focus:outline-hidden focus:ring-2 focus:ring-ring focus:ring-offset-2",
        variants: {
            variant: {
                Default: "border-transparent bg-primary text-primary-foreground shadow hover:bg-primary/80",
                Secondary: "border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80",
                Accent: "border-transparent bg-accent text-accent-foreground hover:bg-accent/80",
                Muted: "border-transparent bg-muted text-muted-foreground hover:bg-muted/80",
                Destructive: "border-transparent bg-destructive text-destructive-foreground shadow hover:bg-destructive/80",
                Outline: "text-foreground",
                Success: "border-transparent bg-success text-success-foreground hover:bg-success/80",
                Warning: "border-transparent bg-warning text-warning-foreground hover:bg-warning/80",
            },
            size: {
                Default: "px-2.5 py-0.5 text-xs",
                Sm: "px-1.5 py-0.5 text-[10px]",
                Lg: "px-3 py-1 text-sm",
            }
        },
        component: {
            element: span
        }
    }
}
