
struct OpalAppRenderState {
	// ...
}

#[derive(Default, Clone)]
struct OpalAppInputState {
	keyboard_scancode_state: FastHashMap<ScanCode, bool>,
}

struct OpalApp {
	render_state: Option<OpalAppRenderState>,
	input_state: OpalAppInputState,
}

impl OpalApp {
	pub fn new() -> Self {
		Self {
			render_state: None
			input_state: OpalAppInputState::default(),
		}
	}

	fn is_keycode_down(&mut self, code: VirtualKeyCode) -> bool {
		self.input_state
			.keyboard_keycode_state
			.get(&code)
			.map_or(false, |v| *v)
	}
}

// impl rend3_framework::App for OpalApp {
impl OpalApp {

	/// Called right before the window is made visible.
	fn setup(
		&mut self
	) {
		// render state is initialized here instead of the OpalApp constructor
		// which is why it is Optional.
		self.render_state = Some(OpalAppRenderState {
			// ...
		});
	}

	/// override handle_event from App
	fn handle_event(
		&mut self
	) {

		// Rust error:
		// cannot borrow `*self` as mutable more than once at a time.

		// Rust error: first mutable borrow occurs here
		let render_state = self.render_state.as_mut().unwrap();

		// Rust error: second mutable borrow occurs here
		if self.is_keycode_down(VirtualKeyCode::W) {

			// Rust error: first borrow later used here
			render_state.camera_data.view = render_state.camera_data.view
						* Mat4::from_translation(Vec3::new(0.0, 0.0, -0.1))
			// ...
		}
	}
}
