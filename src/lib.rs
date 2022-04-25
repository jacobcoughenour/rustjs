use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use glam::{DVec2, EulerRot, Mat3A, Mat4, UVec2, Vec3, Vec3A, Vec4};
use winit::event::DeviceEvent;
use winit::event::WindowEvent as WinitWindowEvent;
use winit::event::{ElementState, ScanCode, VirtualKeyCode};
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowBuilder};

use egui_winit_platform::{Platform, PlatformDescriptor};
use rend3::graph::RenderGraph;
use rend3::types::{
	Camera, CameraProjection, DirectionalLight, DirectionalLightHandle, Handedness, Mesh,
	MeshBuilder, Object, ObjectHandle, ObjectMeshKind, SampleCount, Surface, TextureFormat,
};
use rend3::util::output::OutputFrame;
use rend3::util::typedefs::FastHashMap;
use rend3::Renderer;
use rend3_egui::EguiRenderRoutine;
use rend3_framework::{DefaultRoutines, Event};
use rend3_routine::base::BaseRenderGraph;
use rend3_routine::pbr::{AlbedoComponent, PbrMaterial};

use histogram::Histogram;

fn vertex(pos: [f32; 3]) -> Vec3 {
	return Vec3::from(pos);
}

fn create_mesh() -> Mesh {
	let verts = [
		// far side (0.0, 0.0, 1.0)
		vertex([-1.0, -1.0, 1.0]),
		vertex([1.0, -1.0, 1.0]),
		vertex([1.0, 1.0, 1.0]),
		vertex([-1.0, 1.0, 1.0]),
		// near side (0.0, 0.0, -1.0)
		vertex([-1.0, 1.0, -1.0]),
		vertex([1.0, 1.0, -1.0]),
		vertex([1.0, -1.0, -1.0]),
		vertex([-1.0, -1.0, -1.0]),
		// right side (1.0, 0.0, 0.0)
		vertex([1.0, -1.0, -1.0]),
		vertex([1.0, 1.0, -1.0]),
		vertex([1.0, 1.0, 1.0]),
		vertex([1.0, -1.0, 1.0]),
		// left side (-1.0, 0.0, 0.0)
		vertex([-1.0, -1.0, 1.0]),
		vertex([-1.0, 1.0, 1.0]),
		vertex([-1.0, 1.0, -1.0]),
		vertex([-1.0, -1.0, -1.0]),
		// top (0.0, 1.0, 0.0)
		vertex([1.0, 1.0, -1.0]),
		vertex([-1.0, 1.0, -1.0]),
		vertex([-1.0, 1.0, 1.0]),
		vertex([1.0, 1.0, 1.0]),
		// bottom (0.0, -1.0, 0.0)
		vertex([1.0, -1.0, 1.0]),
		vertex([-1.0, -1.0, 1.0]),
		vertex([-1.0, -1.0, -1.0]),
		vertex([1.0, -1.0, -1.0]),
	];

	let indices: &[u32] = &[
		0, 1, 2, 2, 3, 0, // far
		4, 5, 6, 6, 7, 4, // near
		8, 9, 10, 10, 11, 8, // right
		12, 13, 14, 14, 15, 12, // left
		16, 17, 18, 18, 19, 16, // top
		20, 21, 22, 22, 23, 20, // bottom
	];

	MeshBuilder::new(verts.to_vec(), Handedness::Left)
		.with_indices(indices.to_vec())
		.build()
		.unwrap()
}

#[derive(Default)]
struct OpalAppRenderStats {
	frame_count: u64,
	sample_duration: f32,
	min_frame_time: f32,
	max_frame_time: f32,
	avg_frame_time: f32,
}

struct OpalAppRenderState {
	// scene handles
	object: ObjectHandle,
	directional_light: DirectionalLightHandle,

	camera_pos: Vec3A,
	camera_pitch: f32,
	camera_yaw: f32,

	// egui
	egui_routine: EguiRenderRoutine,
	egui_platform: Platform,

	// rendering
	last_frame_time: Instant,
	start_time: Instant,
	last_capture_time: Instant,
	frame_times: Histogram,
	stats: OpalAppRenderStats,

	input: OpalAppInputManager,
}

#[derive(Default, Clone)]
struct OpalAppInputState {
	keyboard_scancode_state: FastHashMap<ScanCode, bool>,
	keyboard_keycode_state: FastHashMap<VirtualKeyCode, bool>,
	mouse_delta: DVec2,
}

#[derive(Default, Clone)]
struct OpalAppInputManager {
	input_state: OpalAppInputState,
	prev_input_state: OpalAppInputState,
}

impl OpalAppInputManager {
	pub fn push_state(&mut self) {
		self.prev_input_state = self.input_state.clone();
	}

	pub fn handle_event<T>(&mut self, event: &Event<T>) {
		match event {
			Event::WindowEvent {
				event: WinitWindowEvent::KeyboardInput { input, .. },
				..
			} => {
				self.input_state.keyboard_scancode_state.insert(
					input.scancode,
					match input.state {
						ElementState::Pressed => true,
						ElementState::Released => false,
					},
				);
				if input.virtual_keycode.is_some() {
					self.input_state.keyboard_keycode_state.insert(
						input.virtual_keycode.unwrap(),
						match input.state {
							ElementState::Pressed => true,
							ElementState::Released => false,
						},
					);
				}
			}
			Event::DeviceEvent {
				event: DeviceEvent::MouseMotion {
					delta: (delta_x, delta_y),
					..
				},
				..
			} => {
				self.input_state.mouse_delta = DVec2::new(*delta_x, *delta_y);
			}
			_ => {}
		}
	}

	#[inline]
	fn is_pressed<K, H: BuildHasher>(map: &HashMap<K, bool, H>, code: &K) -> bool
	where
		K: Eq + core::hash::Hash,
	{
		map.get(code).map_or(false, |v| *v)
	}

	#[inline]
	fn is_just_pressed<K, H: BuildHasher>(
		prev_map: &HashMap<K, bool, H>,
		map: &HashMap<K, bool, H>,
		code: &K,
	) -> bool
	where
		K: Eq + core::hash::Hash,
	{
		Self::is_pressed(map, code) && !Self::is_pressed(prev_map, code)
	}

	#[inline]
	fn is_just_released<K, H: BuildHasher>(
		prev_map: &HashMap<K, bool, H>,
		map: &HashMap<K, bool, H>,
		code: &K,
	) -> bool
	where
		K: Eq + core::hash::Hash,
	{
		Self::is_just_pressed(map, prev_map, code)
	}

	#[inline]
	pub fn is_keycode_down(&mut self, code: &VirtualKeyCode) -> bool {
		Self::is_pressed(&self.input_state.keyboard_keycode_state, code)
	}

	#[inline]
	pub fn is_keycode_just_pressed(&mut self, code: &VirtualKeyCode) -> bool {
		Self::is_just_pressed(
			&self.prev_input_state.keyboard_keycode_state,
			&self.input_state.keyboard_keycode_state,
			code,
		)
	}

	#[inline]
	pub fn is_keycode_just_released(&mut self, code: &VirtualKeyCode) -> bool {
		Self::is_just_released(
			&self.prev_input_state.keyboard_keycode_state,
			&self.input_state.keyboard_keycode_state,
			code,
		)
	}
}

struct OpalApp {
	render_state: Option<OpalAppRenderState>,
}

const SAMPLE_COUNT: SampleCount = SampleCount::One;

impl OpalApp {
	pub fn new() -> Self {
		Self { render_state: None }
	}
}

impl rend3_framework::App for OpalApp {
	const HANDEDNESS: Handedness = Handedness::Left;

	fn sample_count(&self) -> SampleCount {
		SAMPLE_COUNT
	}

	/// Called right before the window is made visible.
	fn setup(
		&mut self,
		window: &Window,
		renderer: &Arc<Renderer>,
		routines: &Arc<DefaultRoutines>,
		surface_format: TextureFormat,
	) {
		let window_size = window.inner_size();

		// setup egui
		let egui_routine = EguiRenderRoutine::new(
			renderer,
			surface_format,
			SAMPLE_COUNT,
			window_size.width,
			window_size.height,
			window.scale_factor() as f32,
		);

		// integrate with winit
		let egui_platform = Platform::new(PlatformDescriptor {
			physical_width: window_size.width as u32,
			physical_height: window_size.height as u32,
			scale_factor: window.scale_factor(),
			font_definitions: egui::FontDefinitions::default(),
			style: Default::default(),
		});

		// create a cube
		let object = Object {
			mesh_kind: ObjectMeshKind::Static(renderer.add_mesh(create_mesh())),
			material: renderer.add_material(PbrMaterial {
				albedo: AlbedoComponent::Value(Vec4::new(0.0, 0.5, 0.5, 1.0)),
				..PbrMaterial::default()
			}),
			transform: Mat4::IDENTITY,
		};

		// add the mesh object to the scene and keep the handle for it.
		let object = renderer.add_object(object);

		let directional_light = renderer.add_directional_light(DirectionalLight {
			color: Vec3::ONE,
			intensity: 10.0,
			direction: Vec3::new(-1.0, -4.0, 2.0),
			distance: 400.0,
		});

		self.render_state = Some(OpalAppRenderState {
			object,
			directional_light,
			camera_pos: Vec3A::new(3.0, 3.0, -5.0),
			camera_pitch: 0.55,
			camera_yaw: -0.5,
			egui_routine,
			egui_platform,
			last_frame_time: Instant::now(),
			start_time: Instant::now(),
			last_capture_time: Instant::now(),
			frame_times: Histogram::new(),
			stats: OpalAppRenderStats::default(),
			input: OpalAppInputManager::default(),
		});
	}

	/// The main app window event handler
	fn handle_event(
		&mut self,
		window: &Window,
		renderer: &Arc<Renderer>,
		routines: &Arc<DefaultRoutines>,
		base_rendergraph: &BaseRenderGraph,
		surface: Option<&Arc<Surface>>,
		resolution: UVec2,
		event: Event<'_, ()>,
		control_flow: impl FnOnce(ControlFlow),
	) {
		// get the render state object
		let render_state = self.render_state.as_mut().unwrap();

		// pass winit events to egui platform integration
		render_state.egui_platform.handle_event(&event);

		// pass events to input manager
		render_state.input.handle_event(&event);

		match event {
			// OS events
			Event::WindowEvent { event, .. } => match event {
				// close window button clicked
				WinitWindowEvent::CloseRequested => {
					control_flow(ControlFlow::Exit);
				}
				WinitWindowEvent::Resized(size) => {
					render_state.egui_routine.resize(
						size.width,
						size.height,
						window.scale_factor() as f32,
					);
				}
				_ => {}
			},
			// logic loop
			Event::MainEventsCleared => {
				// get frame time
				let now = Instant::now();
				let delta_time = now - render_state.last_frame_time;

				render_state
					.frame_times
					.increment(delta_time.as_micros() as u64)
					.unwrap();

				let time_since_last_second = now - render_state.last_capture_time;
				if time_since_last_second > Duration::from_secs(1) {
					// capture stats
					render_state.stats = OpalAppRenderStats {
						frame_count: render_state.frame_times.entries(),
						sample_duration: time_since_last_second.as_secs_f32(),
						min_frame_time: render_state.frame_times.minimum().unwrap() as f32 / 1000.0,
						max_frame_time: render_state.frame_times.maximum().unwrap() as f32 / 1000.0,
						avg_frame_time: render_state.frame_times.mean().unwrap() as f32 / 1000.0,
					};
					render_state.last_capture_time = now;
					render_state.frame_times.clear();
				}

				render_state.last_frame_time = now;

				if (render_state
					.input
					.is_keycode_just_pressed(&VirtualKeyCode::Escape))
				{
					control_flow(ControlFlow::Exit);
					return;
				}

				let rotation = Mat3A::from_euler(
					glam::EulerRot::XYZ,
					-render_state.camera_pitch,
					-render_state.camera_yaw,
					0.0,
				)
				.transpose();
				let forward = -rotation.z_axis;
				let up = rotation.y_axis;
				let side = -rotation.x_axis;

				let velocity = 10.0 * delta_time.as_secs_f32();

				if render_state.input.is_keycode_down(&VirtualKeyCode::W) {
					render_state.camera_pos -= forward * velocity;
				}
				if render_state.input.is_keycode_down(&VirtualKeyCode::S) {
					render_state.camera_pos += forward * velocity;
				}
				if render_state.input.is_keycode_down(&VirtualKeyCode::A) {
					render_state.camera_pos += side * velocity;
				}
				if render_state.input.is_keycode_down(&VirtualKeyCode::D) {
					render_state.camera_pos -= side * velocity;
				}

				if render_state.input.is_keycode_down(&VirtualKeyCode::E) {
					// render_state.camera_pos += up * velocity;
					render_state.camera_pos += Vec3A::new(0.0, velocity, 0.0);
				}
				if render_state.input.is_keycode_down(&VirtualKeyCode::C) {
					// render_state.camera_pos -= up * velocity;
					render_state.camera_pos -= Vec3A::new(0.0, velocity, 0.0);
				}

				// request a redraw of the scene
				window.request_redraw();

				// reset input manager for next frame
				render_state.input.push_state();
			}

			// render loop
			Event::RedrawRequested(_) => {
				render_state
					.egui_platform
					.update_time(render_state.start_time.elapsed().as_secs_f64());
				render_state.egui_platform.begin_frame();

				let ctx = render_state.egui_platform.context();
				egui::Window::new("stats").resizable(true).show(&ctx, |ui| {
					ui.label(format!(
						"{:0>5} frames over {:0>5.2}s.",
						render_state.stats.frame_count, render_state.stats.sample_duration
					));
					egui::Grid::new("my_grid")
						.num_columns(2)
						.spacing([40.0, 4.0])
						.striped(true)
						.show(ui, |ui| {
							ui.label("avg");
							ui.label(format!("{:0>5.2}ms", render_state.stats.avg_frame_time));
							ui.end_row();
							ui.label("min");
							ui.label(format!("{:0>5.2}ms", render_state.stats.min_frame_time));
							ui.end_row();
							ui.label("max");
							ui.label(format!("{:0>5.2}ms", render_state.stats.max_frame_time));
							ui.end_row();
							ui.label("pos");
							ui.label(format!(
								"x{:0>5.2} y{:0>5.2} z{:0>5.2}",
								render_state.camera_pos.x,
								render_state.camera_pos.y,
								render_state.camera_pos.z
							));
						});
				});

				let (_output, paint_commands) = render_state.egui_platform.end_frame(Some(window));
				let paint_jobs = render_state
					.egui_platform
					.context()
					.tessellate(paint_commands);

				let input = rend3_egui::Input {
					clipped_meshes: &paint_jobs,
					context: render_state.egui_platform.context(),
				};

				let frame = OutputFrame::Surface {
					surface: Arc::clone(surface.unwrap()),
				};

				let view = Mat4::from_euler(
					glam::EulerRot::XYZ,
					-render_state.camera_pitch,
					-render_state.camera_yaw,
					0.0,
				);
				let view = view * Mat4::from_translation((-render_state.camera_pos).into());

				renderer.set_camera_data(Camera {
					projection: CameraProjection::Perspective {
						vfov: 60.0,
						near: 0.1,
					},
					view,
				});

				let (cmd_bufs, ready) = renderer.ready();

				// lock routines
				let pbr_routine = rend3_framework::lock(&routines.pbr);
				let tonemapping_routine = rend3_framework::lock(&routines.tonemapping);

				// build rendergraph
				let mut graph = RenderGraph::new();

				base_rendergraph.add_to_graph(
					&mut graph,
					&ready,
					&pbr_routine,
					None,
					&tonemapping_routine,
					resolution,
					SAMPLE_COUNT,
					Vec4::ZERO,
					// Vec4::new(0.1, 0.05, 0.1, 1.0),
				);

				let surface = graph.add_surface_texture();
				render_state
					.egui_routine
					.add_to_graph(&mut graph, input, surface);

				graph.execute(renderer, frame, cmd_bufs, &ready);

				control_flow(ControlFlow::Poll);
			}

			// ignore the rest
			_ => {}
		}
	}
}

pub fn main() {
	let app = OpalApp::new();
	rend3_framework::start(app, WindowBuilder::new().with_title("Opal Test"));
}
