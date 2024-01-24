use chips::fairchild_f8;

// The way this sound system works is by saving multiple buffers and feeding them to the browser at the right time.
//
// | Sound audio_buffer 1 | Sound audio_buffer 2 | Sound audio_buffer 3 |
//                  ^
//                  |
//              Ended event
//
// At the ended event of sound audio_buffer 1, sound audio_buffer 2 should already start playing.
// While sound audio_buffer 2 is playing, build up audio_buffer 3 and schedule it.
// When sound audio_buffer 2 has ended, we start building up audio_buffer 4.
//
// So this way, we are always planning for the next sound audio_buffer, while a current one is playing.
// We should never be in a situation where nothing is playing.
//
// But what to do if we run into this situation? For example, when the browser is just started, there is no sound. Or if CPU busy, and we don't have enough time to compute the next audio_buffer?
// Then this happens:
// 1. There is nothing playing.
// 2. There is no ending event.
// 3. The only audio_buffer we have, is the current one.
// So, how are we supposed to "jumpstart" this?
// If we timeout from an ending event, we should rebuild audio_buffer 1 and 2 asap (don't sleep between 1 and 2).

//cpu runs at 500 ns per clock tick
const TICKS_PER_SECOND: u64 = 2_000_000; // Each tick takes 500 ns. 1 / 0.0000005 = 2_000_000
const FRAMES_PER_SECOND: usize = 50;

pub(super) struct Sound {
  audio_context: web_sys::AudioContext,
  audio_buffer: Vec<f32>,  //The size of this is sample_rate / frames per second to get the number of samples per frame. 48,000 samples rate / 50 frames per second = 960 samples per frame.
  current_frequency: u64,
  total_clock_ticks: u64,
  ticks_since_freq_start: u64,  //Count how many ticks we have passed for the current frequency.
  sample_rate: u64, //Normally 48,000
  start_time: f64,
  refresh_count: usize,
  current_audio: web_sys::AudioBufferSourceNode,
  previous_audio: web_sys::AudioBufferSourceNode,
  restarting: bool,
}


impl Sound {
  pub fn new() -> Self {
    let audio_context = web_sys::AudioContext::new().expect("Failed to create AudioContext object");
    let sample_rate = audio_context.sample_rate() as u64;
    let start_time = audio_context.current_time();
    let current_audio = audio_context.create_buffer_source().unwrap();
    let previous_audio = audio_context.create_buffer_source().unwrap();
    Self {
      audio_context,
      audio_buffer: vec![0.0; sample_rate as usize / FRAMES_PER_SECOND], //48,000 samples rate / 50 frames per second = 960 samples per frame.
      current_frequency: 0,
      total_clock_ticks: 0,
      ticks_since_freq_start: 0,
      sample_rate,
      start_time,
      refresh_count: 0,
      current_audio,
      previous_audio,
      restarting: false,
    }
  }

  /// Buffers sound. Returns true once we fill up the sound audio_buffer, so the refresh can occur.
  pub fn run_cycle(&mut self, board: &fairchild_f8::Board, clock_ticks: usize) -> bool {
    self.total_clock_ticks += clock_ticks as u64;
    let audio_buffer_index = (self.total_clock_ticks * self.sample_rate / TICKS_PER_SECOND) as usize;
    if audio_buffer_index >= self.audio_buffer.len() {
      return true;
    }
    let freq = match board.read_port(5) >> 6 {
      0b01 => 1000,
      0b10 => 500,
      0b11 => 120,
      _ => 0,
    };
    if self.current_frequency != freq {
      self.current_frequency = freq;
      self.ticks_since_freq_start = 0;
    } else {
      self.ticks_since_freq_start += clock_ticks as u64;
    }
    if freq != 0 {
      let audio_samples_since_freq_start = self.ticks_since_freq_start * self.sample_rate / TICKS_PER_SECOND;
      let period = self.sample_rate / freq;
      self.audio_buffer[audio_buffer_index] = if audio_samples_since_freq_start % period < period / 2 { 1.0 } else { -1.0 };
    }
    false
  }
  
  //Schedules the future audio to play. If no audio is playing currently, it will prepare the previous and current audio too.
  pub async fn run_refresh_cycle(&mut self) {
    let channel_buffer = self.audio_context.create_buffer(1, self.audio_buffer.len() as u32, self.sample_rate as f32).unwrap();
    channel_buffer.copy_to_channel(&self.audio_buffer, 0).unwrap();

    //Create a audio_buffer source for our data
    let future_audio = self.audio_context.create_buffer_source().unwrap();
    future_audio.set_buffer(Some(&channel_buffer));

    //Connect our graph
    future_audio.connect_with_audio_node(&self.audio_context.destination()).unwrap();
    
    future_audio.start_with_when(self.start_time + self.refresh_count as f64 / FRAMES_PER_SECOND as f64).expect("Couldn't schedule sound"); //Schedule the buffered sound
    
    //When restarting, we do not sleep, or wait for anything. previous_audio should be playing now.
    //So I should schedule current_audio to play without sleeping, so we can prepare the future audio.
    if self.restarting {
      self.restarting = false;
      self.refresh_count += 1;
      self.current_audio = future_audio;
    } else {
      if !finish_audio_or_timeout(&self.previous_audio).await {
        //Oh no, we timed out.
        //This means there is something wrong with our buffers.
        //Lets reinitialize everything and rebuild the buffers.
        self.refresh_count = 0;
        self.previous_audio = future_audio;
        self.restarting = true;
      } else {
        self.refresh_count += 1;
        self.previous_audio = std::mem::replace(&mut self.current_audio, future_audio);
      }
    }
    //Get ready for the next sound, cleaning the audio_buffer and reset total_clock_ticks to reset the audio_buffer_index.
    self.audio_buffer.fill(0.0);
    self.total_clock_ticks = 0;
  }
}


use futures::FutureExt; // for `.fuse()`
async fn finish_audio_or_timeout(audio_node: &web_sys::AudioBufferSourceNode) -> bool {
  let audio_event = finish_audio(audio_node).fuse();
  let timeout = sleep(1000 * 2 / FRAMES_PER_SECOND as i32).fuse();  //Timeout 

  futures::pin_mut!(audio_event, timeout);

  futures::select! {
    _ = audio_event => {
      // The `ended` event was triggered
      true
    },
    _ = timeout => {
      // The timeout occurred
      false
    }
  }
}

use wasm_bindgen::closure;
use std::sync;
use wasm_bindgen::JsCast; // for unchecked_ref()

// We are waiting for this sound to finish. This is used instead of sleeping. It finishes once it plays through the 1 / FRAMES_PER_SECOND duration.
async fn finish_audio(audio_node: &web_sys::AudioBufferSourceNode) {
  let (sender, receiver) = futures::channel::oneshot::channel::<()>();
  let sender = sync::Arc::new(sync::Mutex::new(Some(sender)));

  let closure = closure::Closure::wrap(Box::new(move || {
    let mut sender = sender.lock().unwrap();
    if let Some(s) = sender.take() {
      let _ = s.send(());
    }
  }) as Box<dyn FnMut()>);

  audio_node.set_onended(Some(closure.as_ref().unchecked_ref()));
  closure.forget();

  let _ = receiver.await.unwrap();
}

/// A trick to get browsers to "sleep" by awaiting a set_timeout
// This is used as a backup in case sound it not playing. We sleep instead of waiting for the sound to finish.
async fn sleep(milliseconds: i32) {
  let promise = js_sys::Promise::new(&mut |resolve, _| {
    web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, milliseconds).unwrap();
  });
  let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}