use glam::{EulerRot, Mat4, UVec2, Vec3, Vec4};
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowBuilder};

use std::sync::Arc;

use rend3::graph::RenderGraph;
use rend3::types::{
	Camera, CameraProjection, DirectionalLight, DirectionalLightHandle, Handedness, Material, Mesh,
	MeshBuilder, Object, ObjectHandle, ObjectMeshKind, SampleCount, Surface, TextureFormat,
};
use rend3::util::output::OutputFrame;
use rend3::Renderer;
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

#[derive(Default)]
struct OpalApp {
	object_handle: Option<ObjectHandle>,
	directional_light_handle: Option<DirectionalLightHandle>,
}

const SAMPLE_COUNT: SampleCount = SampleCount::One;

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
		// create a cube mesh
		let mesh = create_mesh();
		let mesh_handle = renderer.add_mesh(mesh);

		// create a material
		let material = PbrMaterial {
			albedo: AlbedoComponent::Value(Vec4::new(0.0, 0.5, 0.5, 1.0)),
			..PbrMaterial::default()
		};
		let material_handle = renderer.add_material(material);

		// bring it all together into a mesh object
		let object = Object {
			mesh_kind: ObjectMeshKind::Static(mesh_handle),
			material: material_handle,
			transform: Mat4::IDENTITY,
		};

		// add the mesh object to the scene and keep the handle for it.
		self.object_handle = Some(renderer.add_object(object));

		let view_pos = Vec3::new(3.0, 3.0, -5.0);
		let view =
			Mat4::from_euler(EulerRot::XYZ, -0.55, 0.5, 0.0) * Mat4::from_translation(-view_pos);

		renderer.set_camera_data(Camera {
			projection: CameraProjection::Perspective {
				vfov: 60.0,
				near: 0.1,
			},
			view,
		});

		self.directional_light_handle = Some(renderer.add_directional_light(DirectionalLight {
			color: Vec3::ONE,
			intensity: 10.0,
			direction: Vec3::new(-1.0, -4.0, 2.0),
			distance: 400.0,
		}));
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
		match event {
			// OS events
			Event::WindowEvent {
				// window close button clicked
				event: winit::event::WindowEvent::CloseRequested,
				..
			} => {
				control_flow(ControlFlow::Exit);
			}

			Event::MainEventsCleared => {
				window.request_redraw();
			}

			// render
			Event::RedrawRequested(_) => {
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

				graph.execute(renderer, frame, cmd_bufs, &ready);
			}

			// ignore the rest
			_ => {}
		}
	}
}

pub fn main() {
	let app = OpalApp::default();
	rend3_framework::start(app, WindowBuilder::new().with_title("Opal Test"));
}
