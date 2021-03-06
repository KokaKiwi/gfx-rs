#![feature(phase)]
#![crate_name = "triangle"]

extern crate libc;

extern crate native;
extern crate gl_init;
extern crate gfx;
#[phase(plugin)]
extern crate gfx_macros;
extern crate device;

use device::ApiBackEnd;
use gfx::BackEndHelper;

struct Provider<'a>(&'a gl_init::Window);

impl<'a> device::GlProvider for Provider<'a> {
    fn get_proc_address(&self, name: &str) -> *const libc::c_void {
        let Provider(win) = *self;
        win.get_proc_address(name)
    }
}

#[vertex_format]
struct Vertex {
    pos: [f32, ..2],
    color: [f32, ..3],
}

static VERTEX_SRC: gfx::ShaderSource = shaders! {
GLSL_120: b"
    #version 120
    attribute vec2 pos;
    attribute vec3 color;
    varying vec4 v_Color;
    void main() {
        v_Color = vec4(color, 1.0);
        gl_Position = vec4(pos, 0.0, 1.0);
    }
"
GLSL_150: b"
    #version 150 core
    in vec2 pos;
    in vec3 color;
    out vec4 v_Color;
    void main() {
        v_Color = vec4(color, 1.0);
        gl_Position = vec4(pos, 0.0, 1.0);
    }
"
};

static FRAGMENT_SRC: gfx::ShaderSource = shaders! {
GLSL_120: b"
    #version 120
    varying vec4 v_Color;
    void main() {
        gl_FragColor = v_Color;
    }
"
GLSL_150: b"
    #version 150 core
    in vec4 v_Color;
    out vec4 o_Color;
    void main() {
        o_Color = v_Color;
    }
"
};

// We need to run on the main thread for GLFW, so ensure we are using the `native` runtime. This is
// technically not needed, since this is the default, but it's not guaranteed.
#[start]
fn start(argc: int, argv: *const *const u8) -> int {
     native::start(argc, argv, main)
}

fn main() {
    let window = gl_init::Window::new().unwrap();
    window.set_title("[gl-init] Triangle example #gfx-rs!");
    unsafe { window.make_current() };
    let (w, h) = window.get_inner_size().unwrap();

    let mut backend = device::gl::GlBackEnd::new(&Provider(&window));
    let frontend = backend.create_frontend(w as u16, h as u16).unwrap();

    let state = gfx::DrawState::new();
    let vertex_data = vec![
        Vertex { pos: [ -0.5, -0.5 ], color: [1.0, 0.0, 0.0] },
        Vertex { pos: [ 0.5, -0.5 ], color: [0.0, 1.0, 0.0]  },
        Vertex { pos: [ 0.0, 0.5 ], color: [0.0, 0.0, 1.0]  }
    ];
    let mesh = backend.create_mesh(vertex_data);
    let program = backend.link_program((), VERTEX_SRC.clone(), FRAGMENT_SRC.clone())
                         .unwrap();

    let mut list = frontend.create_drawlist();
    list.clear(
        gfx::ClearData {
            color: Some(gfx::Color([0.3, 0.3, 0.3, 1.0])),
            depth: None,
            stencil: None,
        },
        frontend.get_main_frame()
    );
    list.draw(&mesh, mesh.get_slice(), frontend.get_main_frame(), &program, &state)
        .unwrap();

    'main: loop {
        // quit when Esc is pressed.
        for event in window.poll_events() {
            match event {
                gl_init::KeyboardInput(_, _, Some(gl_init::Escape), _) => break 'main,
                gl_init::Closed => break 'main,
                _ => {},
            }
        }
        backend.submit(list.as_slice());
        window.swap_buffers();
    }
}
