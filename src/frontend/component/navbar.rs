use leptos::prelude::*;

#[component]
pub fn View() -> impl IntoView {
    view! {
        // navbar
        <nav>
            <div class="flex flex-row gap-4 p-8 bg-[#1d2832] text-gray-200">
                <a href="/">"Strichliste"</a>
                <a href="/articles">"Article list"</a>
                <a href="/split_cost">"Split cost"</a>
            </div>
        </nav>
    }
}
