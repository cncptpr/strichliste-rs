use std::rc::Rc;

use leptos::{
    leptos_dom::logging::console_log,
    prelude::{expect_context, GetUntracked, Set},
    task::spawn_local,
};
use reactive_stores::{Store, StoreField};

use crate::{
    frontend::model::money_args::MoneyArgs, model::AudioPlayback, routes::{
        state::{FrontendStore, FrontendStoreStoreFields},
        user::get_item_sound_url,
    }
};

/*
   How to use a callback

   let error_callback = Closure::once(Box::new(move |_: web_sys::Event| {
       // Error playing audio, fall back to TTS
       tts_play(&tts_text);
    }));

    // Attempt to add the error event listener
    let _ = audio.add_event_listener_with_callback("error", error_callback.as_ref().unchecked_ref());
*/

pub fn play_sound(args: Rc<MoneyArgs>, audio_playback: AudioPlayback) {
    use leptos::web_sys::{js_sys, Url};
    spawn_local(async move {
        let store = expect_context::<Store<FrontendStore>>();
        let audio = match args.audio_ref.get_untracked() {
            Some(val) => val,
            None => {
                console_log("Failed to get audio node");
                return;
            }
        };

        let mut cached_sounds = store.cached_sounds().writer().unwrap();

        let url = match cached_sounds.get(&audio_playback) {
            Some(value) => value,
            None => {
                let sound = match get_item_sound_url(audio_playback).await {
                    Ok(value) => value,
                    Err(e) => {
                        args.error.set(format!("Failed to fetch sound: {e}"));
                        return;
                    }
                };

                let sound: &[u8] = &sound;

                let blob = leptos::web_sys::Blob::new_with_u8_array_sequence(&js_sys::Array::of1(
                    &js_sys::Uint8Array::from(sound),
                ))
                .unwrap();

                let url = Url::create_object_url_with_blob(&blob).unwrap();

                _ = cached_sounds.insert(audio_playback, url);

                cached_sounds.get(&audio_playback).unwrap()
            }
        };

        audio.set_src(url);
        match audio.play() {
            Ok(_) => {}
            Err(e) => console_log(&format!("Failed to play audio: {e:#?}")),
        }
    });
}
