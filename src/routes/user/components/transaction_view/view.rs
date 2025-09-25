use std::rc::Rc;

use chrono::{DateTime, Local, Utc};
use leptos::{leptos_dom::logging::console_log, prelude::*};
use leptos_router::hooks::use_params_map;
use leptos_use::{use_infinite_scroll_with_options, UseInfiniteScrollOptions};

use crate::{
    model::{Money, PageResponseParams},
    models::{PageRequestParams, Transaction, TransactionType, UserId},
    routes::user::components::{
        icons::{ArticleBasketIcon, LeftArrowIcon, RightArrowIcon},
        transaction_view::{get_group_members, server::get_user_transactions, UndoTransaction},
    },
};

use crate::routes::user::MoneyArgs;

#[component]
pub fn ShowTransactions(arguments: Rc<MoneyArgs>) -> impl IntoView {
    let params = use_params_map();
    let user_id_string = match params.read_untracked().get("id") {
        Some(s) => s,
        None => {
            return view! { <p class="text-red-500">"Failed to obtain id from url"</p> }.into_any();
        }
    };

    let user_id = match user_id_string.parse::<i64>() {
        Ok(user_id) => UserId(user_id),
        Err(_) => {
            return view! { <p class="text-red-500">"Failed to convert id to a number!"</p> }
                .into_any();
        }
    };

    let previous_transactions_presonse_params: RwSignal<Option<PageResponseParams>> =
        RwSignal::new(None);
    let transaction_data =
        OnceResource::new(get_user_transactions(user_id, PageRequestParams::new(100)));

    let transaction_signal = arguments.transactions;
    let error_signal = arguments.error;
    let money_signal = arguments.money;

    view! {
        <Suspense fallback=move || {
            view! { <p class="text-white text-center p-5">"Loading transactions"</p> }
        }>
            {move || {
                let transactions = match transaction_data.get() {
                    Some(transactions) => transactions,
                    None => {
                        return view! {
                            <p class="text-white bg-red-400 text-center">
                                "Failed to fetch transactions"
                            </p>
                        }
                            .into_any();
                    }
                };
                let mut transactions = match transactions {
                    Ok(transactions) => transactions.items,
                    Err(err) => {
                        let msg = match err {
                            ServerFnError::ServerError(msg) => msg,
                            _ => "Failed to fetch transactions".to_string(),
                        };
                        return view! {
                            <p class="text-white text-center bg-red-400">
                                "Failed to fetch users because: "{msg}
                            </p>
                        }
                            .into_any();
                    }
                };
                transactions.sort_by(|a, b| { b.timestamp.cmp(&a.timestamp) });
                let el = NodeRef::<leptos::html::Div>::new();
                transaction_signal
                    .write()
                    .append(&mut transactions.into_iter().collect::<Vec<Transaction>>());
                Effect::new(move |_| {
                    let _ = use_infinite_scroll_with_options(
                        el,
                        move |_| async move {
                            let next_params = previous_transactions_presonse_params
                                .with_untracked(|p| PageResponseParams::next_params(*p, 100));
                            if let Some(params) = next_params {
                                let mut data = get_user_transactions(user_id, params).await;
                                match data {
                                    Ok(mut data) => {
                                        transaction_signal.update(|d| d.append(&mut data.items));
                                        previous_transactions_presonse_params
                                            .set(Some(data.params));
                                    }
                                    Err(e) => console_log(&e.to_string()),
                                }
                            }
                        },
                        UseInfiniteScrollOptions::default().distance(20.0).interval(1.0),
                    );
                });

                view! {
                    <div class="pl-4 text-[1.25em] h-[800px] w-full overflow-y-scroll" node_ref=el>
                        <For
                            each=move || transaction_signal.get()
                            key=|transaction| (
                                transaction.id,
                                transaction.is_undone_signal.get(),
                                transaction.timestamp,
                            )
                            let:child
                        >
                            {format_transaction(&child, user_id, error_signal, money_signal)}
                        </For>

                    </div>
                }
                    .into_any()
            }}

        </Suspense>
    }
    .into_any()
}

pub fn format_transaction(
    transaction: &Transaction,
    user_id: UserId,
    error_write: RwSignal<String>,
    money_signal: RwSignal<Money>,
) -> impl IntoView {
    let now: DateTime<Utc> = Utc::now();
    let diff = now - transaction.timestamp;

    let undo_action = ServerAction::<UndoTransaction>::new();
    let transaction_id = transaction.id;

    let date_string = format!(
        "{}",
        transaction
            .timestamp
            .with_timezone(&Local)
            .format("%d.%m.%Y %H:%M:%S")
    );

    let undo_signal = transaction.is_undone_signal;

    let money = match transaction.t_type {
        TransactionType::Deposit
        | TransactionType::Received(_)
        | TransactionType::SentAndReceived(_) => transaction.money.value,

        TransactionType::Withdraw | TransactionType::Bought(_) | TransactionType::Sent(_) => {
            -transaction.money.value
        }
    };

    view! {
        <div
            class="grid grid-cols-3 items-center border-t-4 border-gray-300 p-2 text-white"
            class=("line-through", undo_signal.get())
        >
            {match transaction.t_type {
                TransactionType::Withdraw => {
                    view! {
                        <p class="text-red-400">"-"{transaction.money.format_eur()}</p>
                        <p></p>
                    }
                        .into_any()
                }
                TransactionType::Deposit => {

                    view! {
                        <p class="text-green-500">{transaction.money.format_eur_diff()}</p>
                        <p></p>
                    }
                        .into_any()
                }
                TransactionType::Bought(_) => {

                    view! {
                        <p class="text-red-400">"-"{transaction.money.format_eur()}</p>
                        <p class="text-white">
                            <ArticleBasketIcon class="inline" />
                            " "
                            {transaction.description.clone().unwrap_or("".to_string())}
                        </p>
                    }
                        .into_any()
                }
                TransactionType::SentAndReceived(_received_group) => todo!(),
                TransactionType::Received(group) | TransactionType::Sent(group) => {
                    let transaction = transaction.clone();
                    let group_members_resource = OnceResource::new(get_group_members(group.0));
                    let money_value = match transaction.t_type {
                        TransactionType::Received(_) => transaction.money.value,
                        TransactionType::Sent(_) => -transaction.money.value,
                        _ => unreachable!(),
                    };

                    view! {
                        {if money_value < 0 {
                            view! {
                                <p class="text-red-400">"-"{transaction.money.format_eur()}</p>
                            }
                                .into_any()
                        } else {
                            view! {
                                <p class="text-green-500">"+"{transaction.money.format_eur()}</p>
                            }
                                .into_any()
                        }}
                        <Suspense fallback=move || {
                            view! { <p>"Loading users"</p> }
                        }>
                            {move || {
                                let description = transaction
                                    .description
                                    .as_ref()
                                    .map(|val| format!(": {val}"));
                                let fmt_description = |other: String, description: Option<String>| {
                                    other + &description.unwrap_or_default()
                                };
                                group_members_resource
                                    .get()
                                    .map(|group_members| {
                                        match group_members {
                                            Ok(members) => {

                                                view! {
                                                    <p class="text-white flex items-center">
                                                        {if money_value < 0 {
                                                            view! {
                                                                <RightArrowIcon class="w-[2rem]" />
                                                                {fmt_description(members.join(", "), description)}
                                                            }
                                                                .into_any()
                                                        } else {
                                                            view! {
                                                                <LeftArrowIcon class="w-[2rem]" />
                                                                {fmt_description(members.join(", "), description)}
                                                            }
                                                                .into_any()
                                                        }}
                                                    </p>
                                                }
                                                    .into_any()
                                            }
                                            Err(error) => {
                                                let message = match error {
                                                    ServerFnError::ServerError(msg) => msg,
                                                    _ => error.to_string(),
                                                };

                                                view! {
                                                    <p class="text-red-400">
                                                        "Failed to fetch members: "{message}
                                                    </p>
                                                }
                                                    .into_any()
                                            }
                                        }
                                    })
                            }}
                        </Suspense>
                    }
                        .into_any()
                }
            }}
            {move || match undo_signal.get() {
                true => view! { <p class="text-white">{date_string.clone()}</p> }.into_any(),
                false => {
                    if diff.num_minutes() > 2 {

                        // grace period for undoing transactions
                        // if transaction is already undone, only show the date regardless of grace period
                        view! { <p class="text-white">{date_string.clone()}</p> }
                            .into_any()
                    } else {
                        view! {
                            <ActionForm action=undo_action>
                                <input type="hidden" name="user_id" value=user_id.0 />
                                <input type="hidden" name="transaction_id" value=transaction_id />
                                <input type="submit" class="text-white" value="Undo" />
                            </ActionForm>
                        }
                            .into_any()
                    }
                }
            }}
            {move || match undo_action.value().get() {
                None => {}
                Some(response) => {
                    match response {
                        Ok(_) => {
                            undo_signal.set(true);
                            money_signal.update(|value| value.value -= money);
                            console_log("Set signal to true");
                            error_write.set(String::new());
                        }
                        Err(e) => {
                            let msg = match e {
                                ServerFnError::ServerError(msg) => msg,
                                _ => e.to_string(),
                            };
                            error_write.set(msg);
                        }
                    }
                }
            }}
        </div>
    }
}
