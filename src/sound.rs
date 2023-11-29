use boards::fairchild_ves;

pub(super) struct Sound {
  audio_context: web_sys::AudioContext,
  oscillator: web_sys::OscillatorNode,
  current_freq: f32,
}

impl Sound {
  pub fn new() -> Self {
    let audio_context = web_sys::AudioContext::new().expect("Failed to create AudioContext object");
    let oscillator = audio_context.create_oscillator().expect("Failed to create Oscillator object");
    Self {
      current_freq: 0.0,
      audio_context: audio_context,
      oscillator: oscillator,
    }
  }

  /// Start any sound. Making sure I at least start any sound if there is any. The refresh cycle will turn it off.
  pub fn run_cycle(&mut self, board: &fairchild_ves::Board) {
    let freq = match board.read_port(5) >> 6 {
      0b01 => 1000.0,
      0b10 => 500.0,
      0b11 => 120.0,
      _ => 0.0,
    };
    if self.current_freq != freq && freq != 0.0 { //Not turning off the sound, because it might be on for too little time. Turning off will happen on the refresh cycle.
      self.current_freq = freq;
      let _ = self.oscillator.stop(); //This is allowed to fail in case we are stopping before we started.
      if freq != 0.0 {  //Browser hasn't started the oscillator yet. Need to start it over again...
        let oscillator = self.audio_context.create_oscillator().expect("Failed to create Oscillator object");
        oscillator.connect_with_audio_node(&self.audio_context.destination()).expect("Failed to connect Oscillator to AudioContext");
        oscillator.set_type(web_sys::OscillatorType::Square);
        oscillator.frequency().set_value(freq);
        let _ = oscillator.start(); //This is allowed to fail in case we are playing the same thing again.
        self.oscillator = oscillator;
      }
    }
  }
  
  /// Turn off any sound that may have started during the run cycle
  pub fn run_refresh_cycle(&mut self, board: &fairchild_ves::Board) {
    if board.read_port(5) >> 6 == 0 {
      let _ = self.oscillator.stop(); //This is allowed to fail in case we are stopping before we started.
      self.current_freq = 0.0;
    }
  }
}
