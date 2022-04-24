use std::sync::Arc;
use std::time::Instant;

use glam::{EulerRot, Mat4, UVec2, Vec3, Vec4};
use winit::event::WindowEvent as WinitWindowEvent;
use winit::event::{ElementState, VirtualKeyCode};
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

struct OpalAppRenderState {
	// scene handles
	object: ObjectHandle,
	directional_light: DirectionalLightHandle,
	camera_data: Camera,

	// egui
	egui_routine: EguiRenderRoutine,
	egui_platform: Platform,

	// rendering
	last_frame_time: Instant,
	start_time: Instant,
}

struct OpalApp {
	render_state: Option<OpalAppRenderState>,
	// input
	keyboard_input_status: FastHashMap<u32, bool>,
}

const SAMPLE_COUNT: SampleCount = SampleCount::One;

impl OpalApp {
	pub fn new() -> Self {
		Self {
			render_state: None,
			keyboard_input_status: FastHashMap::default(),
		}
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

		// setup camera
		let view_pos = Vec3::new(3.0, 3.0, -5.0);
		let view =
			Mat4::from_euler(EulerRot::XYZ, -0.55, 0.5, 0.0) * Mat4::from_translation(-view_pos);
		let camera_data = Camera {
			projection: CameraProjection::Perspective {
				vfov: 60.0,
				near: 0.1,
			},
			view,
		};
		renderer.set_camera_data(camera_data);

		let directional_light = renderer.add_directional_light(DirectionalLight {
			color: Vec3::ONE,
			intensity: 10.0,
			direction: Vec3::new(-1.0, -4.0, 2.0),
			distance: 400.0,
		});

		self.render_state = Some(OpalAppRenderState {
			object,
			directional_light,
			camera_data,
			egui_routine,
			egui_platform,
			last_frame_time: Instant::now(),
			start_time: Instant::now(),
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

		match event {
			// OS events
			Event::WindowEvent { event, .. } => match event {
				// close window button clicked
				WinitWindowEvent::CloseRequested => {
					control_flow(ControlFlow::Exit);
				}
				// keyboard input
				WinitWindowEvent::KeyboardInput { input, .. } => {
					// key pressed
					if input.state == ElementState::Pressed {
						// esc key quits
						if input.virtual_keycode == Some(VirtualKeyCode::Escape) {
							control_flow(ControlFlow::Exit);
						}

						if input.virtual_keycode == Some(VirtualKeyCode::W) {
							render_state.camera_data.view = render_state.camera_data.view
								* Mat4::from_translation(Vec3::new(0.0, 0.0, -0.1));

							renderer.set_camera_data(render_state.camera_data.clone());
						}
					}
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
				let delta = now - render_state.last_frame_time;

				render_state.last_frame_time = now;

				// request a redraw of the scene
				window.request_redraw();
			}

			// render loop
			Event::RedrawRequested(_) => {
				render_state
					.egui_platform
					.update_time(render_state.start_time.elapsed().as_secs_f64());
				render_state.egui_platform.begin_frame();

				let ctx = render_state.egui_platform.context();
				egui::Window::new("test").resizable(true).show(&ctx, |ui| {
					ui.label("hello world");
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
