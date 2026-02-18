use leptos::prelude::*;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconName {
    User,
    Settings,
    Box,
    Circle,
    Square,
    Cylinder,
    Cone,
    Torus,
    Move,
    RotateCw,
    Scale,
    Copy,
    Trash2,
    Link,
    Grid3x3,
    Layers,
    Ruler,
    Gauge,
    Eye,
    File,
    Image,
    Database,
    MousePointer,
    Hand,
    ChevronDown,
    Search,
    Filter,
    EyeOff,
    ChevronRight,
    FileText,
    Bookmark,
    Compass,
    Link2,
    PenTool,
    Folder,
    MoreVertical,
    MousePointer2,
    ZoomIn,
    ZoomOut,
    Maximize2,
    Terminal,
    ChevronUp,
    X,
    Check,
    AlertTriangle,
    Info,
    Keyboard,
    Command,
    Calendar,
    Package,
    SkipBack,
    Play,
    SkipForward,
    ChevronLeft,
}

fn icon_svg_body(name: IconName) -> &'static str {
    match name {
        IconName::User => {
            r#"<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" />
<circle cx="12" cy="7" r="4" />"#
        }
        IconName::Settings => {
            r#"<path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915" />
<circle cx="12" cy="12" r="3" />"#
        }
        IconName::Box => {
            r#"<path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z" />
<path d="m3.3 7 8.7 5 8.7-5" />
<path d="M12 22V12" />"#
        }
        IconName::Circle => r#"<circle cx="12" cy="12" r="10" />"#,
        IconName::Square => r#"<rect width="18" height="18" x="3" y="3" rx="2" />"#,
        IconName::Cylinder => {
            r#"<ellipse cx="12" cy="5" rx="9" ry="3" />
<path d="M3 5v14a9 3 0 0 0 18 0V5" />"#
        }
        IconName::Cone => {
            r#"<path d="m20.9 18.55-8-15.98a1 1 0 0 0-1.8 0l-8 15.98" />
<ellipse cx="12" cy="19" rx="9" ry="3" />"#
        }
        IconName::Torus => {
            r#"<ellipse cx="12" cy="11" rx="3" ry="2" />
<ellipse cx="12" cy="12.5" rx="10" ry="8.5" />"#
        }
        IconName::Move => {
            r#"<path d="M12 2v20" />
<path d="m15 19-3 3-3-3" />
<path d="m19 9 3 3-3 3" />
<path d="M2 12h20" />
<path d="m5 9-3 3 3 3" />
<path d="m9 5 3-3 3 3" />"#
        }
        IconName::RotateCw => {
            r#"<path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8" />
<path d="M21 3v5h-5" />"#
        }
        IconName::Scale => {
            r#"<path d="M12 3v18" />
<path d="m19 8 3 8a5 5 0 0 1-6 0zV7" />
<path d="M3 7h1a17 17 0 0 0 8-2 17 17 0 0 0 8 2h1" />
<path d="m5 8 3 8a5 5 0 0 1-6 0zV7" />
<path d="M7 21h10" />"#
        }
        IconName::Copy => {
            r#"<rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
<path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />"#
        }
        IconName::Trash2 => {
            r#"<path d="M10 11v6" />
<path d="M14 11v6" />
<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
<path d="M3 6h18" />
<path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />"#
        }
        IconName::Link => {
            r#"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
<path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />"#
        }
        IconName::Grid3x3 => {
            r#"<rect width="18" height="18" x="3" y="3" rx="2" />
<path d="M3 9h18" />
<path d="M3 15h18" />
<path d="M9 3v18" />
<path d="M15 3v18" />"#
        }
        IconName::Layers => {
            r#"<path d="M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z" />
<path d="M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12" />
<path d="M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17" />"#
        }
        IconName::Ruler => {
            r#"<path d="M21.3 15.3a2.4 2.4 0 0 1 0 3.4l-2.6 2.6a2.4 2.4 0 0 1-3.4 0L2.7 8.7a2.41 2.41 0 0 1 0-3.4l2.6-2.6a2.41 2.41 0 0 1 3.4 0Z" />
<path d="m14.5 12.5 2-2" />
<path d="m11.5 9.5 2-2" />
<path d="m8.5 6.5 2-2" />
<path d="m17.5 15.5 2-2" />"#
        }
        IconName::Gauge => {
            r#"<path d="m12 14 4-4" />
<path d="M3.34 19a10 10 0 1 1 17.32 0" />"#
        }
        IconName::Eye => {
            r#"<path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0" />
<circle cx="12" cy="12" r="3" />"#
        }
        IconName::File => {
            r#"<path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z" />
<path d="M14 2v5a1 1 0 0 0 1 1h5" />"#
        }
        IconName::Image => {
            r#"<rect width="18" height="18" x="3" y="3" rx="2" ry="2" />
<circle cx="9" cy="9" r="2" />
<path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21" />"#
        }
        IconName::Database => {
            r#"<ellipse cx="12" cy="5" rx="9" ry="3" />
<path d="M3 5V19A9 3 0 0 0 21 19V5" />
<path d="M3 12A9 3 0 0 0 21 12" />"#
        }
        IconName::MousePointer => {
            r#"<path d="M12.586 12.586 19 19" />
<path d="M3.688 3.037a.497.497 0 0 0-.651.651l6.5 15.999a.501.501 0 0 0 .947-.062l1.569-6.083a2 2 0 0 1 1.448-1.479l6.124-1.579a.5.5 0 0 0 .063-.947z" />"#
        }
        IconName::Hand => {
            r#"<path d="M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2" />
<path d="M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2" />
<path d="M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8" />
<path d="M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15" />"#
        }
        IconName::ChevronDown => r#"<path d="m6 9 6 6 6-6" />"#,
        IconName::Search => {
            r#"<path d="m21 21-4.34-4.34" />
<circle cx="11" cy="11" r="8" />"#
        }
        IconName::Filter => {
            r#"<path d="M10 20a1 1 0 0 0 .553.895l2 1A1 1 0 0 0 14 21v-7a2 2 0 0 1 .517-1.341L21.74 4.67A1 1 0 0 0 21 3H3a1 1 0 0 0-.742 1.67l7.225 7.989A2 2 0 0 1 10 14z" />"#
        }
        IconName::EyeOff => {
            r#"<path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49" />
<path d="M14.084 14.158a3 3 0 0 1-4.242-4.242" />
<path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143" />
<path d="m2 2 20 20" />"#
        }
        IconName::ChevronRight => r#"<path d="m9 18 6-6-6-6" />"#,
        IconName::FileText => {
            r#"<path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z" />
<path d="M14 2v5a1 1 0 0 0 1 1h5" />
<path d="M10 9H8" />
<path d="M16 13H8" />
<path d="M16 17H8" />"#
        }
        IconName::Bookmark => {
            r#"<path d="M17 3a2 2 0 0 1 2 2v15a1 1 0 0 1-1.496.868l-4.512-2.578a2 2 0 0 0-1.984 0l-4.512 2.578A1 1 0 0 1 5 20V5a2 2 0 0 1 2-2z" />"#
        }
        IconName::Compass => {
            r#"<circle cx="12" cy="12" r="10" />
<path d="m16.24 7.76-1.804 5.411a2 2 0 0 1-1.265 1.265L7.76 16.24l1.804-5.411a2 2 0 0 1 1.265-1.265z" />"#
        }
        IconName::Link2 => {
            r#"<path d="M9 17H7A5 5 0 0 1 7 7h2" />
<path d="M15 7h2a5 5 0 1 1 0 10h-2" />
<line x1="8" x2="16" y1="12" y2="12" />"#
        }
        IconName::PenTool => {
            r#"<path d="M15.707 21.293a1 1 0 0 1-1.414 0l-1.586-1.586a1 1 0 0 1 0-1.414l5.586-5.586a1 1 0 0 1 1.414 0l1.586 1.586a1 1 0 0 1 0 1.414z" />
<path d="m18 13-1.375-6.874a1 1 0 0 0-.746-.776L3.235 2.028a1 1 0 0 0-1.207 1.207L5.35 15.879a1 1 0 0 0 .776.746L13 18" />
<path d="m2.3 2.3 7.286 7.286" />
<circle cx="11" cy="11" r="2" />"#
        }
        IconName::Folder => {
            r#"<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />"#
        }
        IconName::MoreVertical => {
            r#"<circle cx="12" cy="12" r="1" />
<circle cx="12" cy="5" r="1" />
<circle cx="12" cy="19" r="1" />"#
        }
        IconName::MousePointer2 => {
            r#"<path d="M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z" />"#
        }
        IconName::ZoomIn => {
            r#"<circle cx="11" cy="11" r="8" />
<line x1="21" x2="16.65" y1="21" y2="16.65" />
<line x1="11" x2="11" y1="8" y2="14" />
<line x1="8" x2="14" y1="11" y2="11" />"#
        }
        IconName::ZoomOut => {
            r#"<circle cx="11" cy="11" r="8" />
<line x1="21" x2="16.65" y1="21" y2="16.65" />
<line x1="8" x2="14" y1="11" y2="11" />"#
        }
        IconName::Maximize2 => {
            r#"<path d="M15 3h6v6" />
<path d="m21 3-7 7" />
<path d="m3 21 7-7" />
<path d="M9 21H3v-6" />"#
        }
        IconName::Terminal => {
            r#"<path d="M12 19h8" />
<path d="m4 17 6-6-6-6" />"#
        }
        IconName::ChevronUp => r#"<path d="m18 15-6-6-6 6" />"#,
        IconName::X => {
            r#"<path d="M18 6 6 18" />
<path d="m6 6 12 12" />"#
        }
        IconName::Check => r#"<path d="M20 6 9 17l-5-5" />"#,
        IconName::AlertTriangle => {
            r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" />
<path d="M12 9v4" />
<path d="M12 17h.01" />"#
        }
        IconName::Info => {
            r#"<circle cx="12" cy="12" r="10" />
<path d="M12 16v-4" />
<path d="M12 8h.01" />"#
        }
        IconName::Keyboard => {
            r#"<path d="M10 8h.01" />
<path d="M12 12h.01" />
<path d="M14 8h.01" />
<path d="M16 12h.01" />
<path d="M18 8h.01" />
<path d="M6 8h.01" />
<path d="M7 16h10" />
<path d="M8 12h.01" />
<rect width="20" height="16" x="2" y="4" rx="2" />"#
        }
        IconName::Command => {
            r#"<path d="M15 6v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3" />"#
        }
        IconName::Calendar => {
            r#"<path d="M8 2v4" />
<path d="M16 2v4" />
<rect width="18" height="18" x="3" y="4" rx="2" />
<path d="M3 10h18" />"#
        }
        IconName::Package => {
            r#"<path d="M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z" />
<path d="M12 22V12" />
<polyline points="3.29 7 12 12 20.71 7" />
<path d="m7.5 4.27 9 5.15" />"#
        }
        IconName::SkipBack => {
            r#"<path d="M17.971 4.285A2 2 0 0 1 21 6v12a2 2 0 0 1-3.029 1.715l-9.997-5.998a2 2 0 0 1-.003-3.432z" />
<path d="M3 20V4" />"#
        }
        IconName::Play => {
            r#"<path d="M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z" />"#
        }
        IconName::SkipForward => {
            r#"<path d="M21 4v16" />
<path d="M6.029 4.285A2 2 0 0 0 3 6v12a2 2 0 0 0 3.029 1.715l9.997-5.998a2 2 0 0 0 .003-3.432z" />"#
        }
        IconName::ChevronLeft => r#"<path d="m15 18-6-6 6-6" />"#,
    }
}

#[component]
pub fn UiIcon(
    name: IconName,
    #[prop(default = 20)] size: u16,
    #[prop(optional, into)] class: MaybeProp<String>,
) -> impl IntoView {
    let class_name = move || class.get().unwrap_or_default();
    let body = icon_svg_body(name);
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class=class_name
            inner_html=body
        ></svg>
    }
}
