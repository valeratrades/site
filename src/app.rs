use leptos::{ev, html::*, prelude::*, svg::svg};
use leptos_meta::{MetaTags, Stylesheet, StylesheetProps, Title, TitleProps, provide_meta_context};
use leptos_routable::prelude::*;
use leptos_router::components::{A, AProps, Router};

use crate::dashboards::{self, DashboardsView};

#[component]
fn TopBar() -> impl IntoView {
	nav().class("flex items-center p-4 bg-gray-800 text-white").child((
		div().class("flex gap-4").child((
			A(AProps {
				href: "/".to_string(),
				children: Box::new(|| view! { "Home" }.into_any()),
				target: None,
				exact: false,
				strict_trailing_slash: false,
				scroll: true,
			})
			.attr("class", "hover:text-blue-300 transition-colors"),
			A(AProps {
				href: "/dashboards".to_string(),
				children: Box::new(|| view! { "Dashboards" }.into_any()),
				target: None,
				exact: false,
				strict_trailing_slash: false,
				scroll: true,
			})
			.attr("class", "hover:text-blue-300 transition-colors"),
			A(AProps {
				href: "/contacts".to_string(),
				children: Box::new(|| view! { "Contacts" }.into_any()),
				target: None,
				exact: false,
				strict_trailing_slash: false,
				scroll: true,
			})
			.attr("class", "hover:text-blue-300 transition-colors"),
		)),
		div().class("ml-auto").child(A(AProps {
			href: "/login".to_string(),
			children: Box::new(|| {
				div()
					.class("w-8 h-8 rounded-full bg-gray-600 flex items-center justify-center hover:bg-gray-500 transition-colors")
					.child(
						svg()
							.attr("viewBox", "0 0 24 24")
							.attr("fill", "none")
							.attr("stroke", "currentColor")
							.attr("stroke-width", "2")
							.class("w-5 h-5")
							.child(leptos::svg::path().attr("d", "M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z")),
					)
					.into_any()
			}),
			target: None,
			exact: false,
			strict_trailing_slash: false,
			scroll: true,
		})),
	))
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
	view! {
		<!DOCTYPE html>
		<html lang="en">
			<head>
				<meta charset="utf-8" />
				<meta name="viewport" content="width=device-width, initial-scale=1" />
				<AutoReload options=options.clone() />
				<HydrationScripts options islands=true />
				<MetaTags />
				// all embedded plotly plots assume this is in scope
				<script src="https://cdn.plot.ly/plotly-3.0.1.min.js"></script>
			</head>
			<body>
				<App />
			</body>
		</html>
	}
}

#[component]
pub fn App() -> impl IntoView {
	provide_meta_context();
	(
		Stylesheet(StylesheetProps {
			id: Some("leptos".to_owned()),
			href: format!("/pkg/{}.css", env!("CARGO_PKG_NAME")),
		}),
		Title(TitleProps {
			formatter: None,
			text: Some("My Site".into()),
		}),
		view! {
			<Router>
				<TopBar />
				<main class="min-h-screen">{move || AppRoutes::routes()}</main>
			</Router>
		},
	)
}

#[derive(Routable)]
#[routes(view_prefix = "", view_suffix = "View", transition = false)]
pub enum AppRoutes {
	#[route(path = "/")]
	Home,
	#[parent_route(path = "/dashboards")]
	Dashboards(dashboards::Routes),
	#[route(path = "/contacts")]
	Contacts,
	#[route(path = "/login")]
	Login,
	#[fallback]
	#[route(path = "/404")]
	NotFound,
}

/// Renders the home page of your application.
#[component]
fn HomeView() -> impl IntoView {
	div().child((
		h1().child("Welcome to Leptos!"),
		HomeButton(),
		p().class("bg-purple-500 text-white p-2 rounded m-2")
			.child("Tailwind check: this should have purple background and rounded corners"), //dbg
	))
}

#[component]
fn ContactsView() -> impl IntoView {
	section().class("p-8 max-w-2xl mx-auto").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Contacts".into()),
		}),
		h1().class("text-3xl font-bold mb-8 text-center").child("Get in Touch"),
		div().class("grid grid-cols-1 sm:grid-cols-2 gap-4").child((
			contact_link("GitHub", "https://github.com/valeratrades", "M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"),
			contact_link("LinkedIn", "https://linkedin.com/in/valeratrades", "M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433c-1.144 0-2.063-.926-2.063-2.065 0-1.138.92-2.063 2.063-2.063 1.14 0 2.064.925 2.064 2.063 0 1.139-.925 2.065-2.064 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z"),
			contact_link("Discord", "https://discord.com/users/valeratrades", "M20.317 4.3698a19.7913 19.7913 0 00-4.8851-1.5152.0741.0741 0 00-.0785.0371c-.211.3753-.4447.8648-.6083 1.2495-1.8447-.2762-3.68-.2762-5.4868 0-.1636-.3933-.4058-.8742-.6177-1.2495a.077.077 0 00-.0785-.037 19.7363 19.7363 0 00-4.8852 1.515.0699.0699 0 00-.0321.0277C.5334 9.0458-.319 13.5799.0992 18.0578a.0824.0824 0 00.0312.0561c2.0528 1.5076 4.0413 2.4228 5.9929 3.0294a.0777.0777 0 00.0842-.0276c.4616-.6304.8731-1.2952 1.226-1.9942a.076.076 0 00-.0416-.1057c-.6528-.2476-1.2743-.5495-1.8722-.8923a.077.077 0 01-.0076-.1277c.1258-.0943.2517-.1923.3718-.2914a.0743.0743 0 01.0776-.0105c3.9278 1.7933 8.18 1.7933 12.0614 0a.0739.0739 0 01.0785.0095c.1202.099.246.1981.3728.2924a.077.077 0 01-.0066.1276 12.2986 12.2986 0 01-1.873.8914.0766.0766 0 00-.0407.1067c.3604.698.7719 1.3628 1.225 1.9932a.076.076 0 00.0842.0286c1.961-.6067 3.9495-1.5219 6.0023-3.0294a.077.077 0 00.0313-.0552c.5004-5.177-.8382-9.6739-3.5485-13.6604a.061.061 0 00-.0312-.0286zM8.02 15.3312c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9555-2.4189 2.157-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.9555 2.4189-2.1569 2.4189zm7.9748 0c-1.1825 0-2.1569-1.0857-2.1569-2.419 0-1.3332.9554-2.4189 2.1569-2.4189 1.2108 0 2.1757 1.0952 2.1568 2.419 0 1.3332-.9460 2.4189-2.1568 2.4189z"),
			contact_link("Twitter", "https://twitter.com/valeratrades", "M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"),
			contact_link("Telegram", "https://t.me/valeratrades", "M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"),
			contact_link("Email", "mailto:valeratrades@gmail.com", "M24 5.457v13.909c0 .904-.732 1.636-1.636 1.636h-3.819V11.73L12 16.64l-6.545-4.91v9.273H1.636A1.636 1.636 0 0 1 0 19.366V5.457c0-2.023 2.309-3.178 3.927-1.964L5.455 4.64 12 9.548l6.545-4.91 1.528-1.145C21.69 2.28 24 3.434 24 5.457z"),
		)),
	))
}

fn contact_link(name: &'static str, href: &'static str, svg_path: &'static str) -> impl IntoView {
	a().attr("href", href)
		.attr("target", "_blank")
		.attr("rel", "noopener noreferrer")
		.class("flex items-center gap-3 p-4 bg-gray-800 hover:bg-gray-700 rounded-lg transition-colors text-white")
		.child((
			svg()
				.attr("viewBox", "0 0 24 24")
				.attr("fill", "currentColor")
				.class("w-6 h-6")
				.child(leptos::svg::path().attr("d", svg_path)),
			span().class("font-medium").child(name),
		))
}

#[component]
fn LoginView() -> impl IntoView {
	section().class("p-4 max-w-md mx-auto mt-8").child((
		Title(TitleProps {
			formatter: None,
			text: Some("Login".into()),
		}),
		h1().class("text-2xl font-bold mb-4").child("Login"),
		p().class("text-gray-600 mb-4").child("Login functionality coming soon"),
	))
}

//dbg
#[island]
fn HomeButton() -> impl IntoView {
	let count = RwSignal::new(0);
	let on_click = move |_| *count.write() += 1;

	button().on(ev::click, on_click).child(move || format!("Click Me: {}", count.read()))
}

#[component]
pub fn NotFoundView() -> impl IntoView {
	div().class("p-4 text-center").child((
		h1().class("text-2xl font-bold"),
		p().child("Sorry, we can't find that page"),
		A(AProps {
			href: "/".to_string(),
			children: Box::new(|| view! { "Go Home" }.into_any()),
			target: None,
			exact: false,
			strict_trailing_slash: false,
			scroll: true,
		})
		.attr("class", "inline-block px-4 py-2 bg-green-500 text-white rounded mt-4"),
	))
}
