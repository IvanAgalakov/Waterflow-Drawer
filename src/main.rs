use std::{
    alloc::System,
    convert::Infallible,
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    result,
    str::Bytes,
};

use egui::{Color32, Pos2, TextEdit, Vec2};
use egui_winit::winit::event::{KeyboardInput, ModifiersState};
use futures::executor::block_on;
use glium::{glutin, implement_vertex, uniform, Display, Frame, Program, Surface};
use image::DynamicImage;
use regex::Regex;
use reqwest::{Client, IntoUrl, Response};
use texture_manager::TextureData;

pub mod texture_manager;

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub(crate) position: [f32; 2],
    pub(crate) tex_coords: [f32; 2],
}
implement_vertex!(Vertex, position, tex_coords);
impl Vertex {
    pub fn as_vector(&self) -> Vec2 {
        Vec2 {
            x: self.position[0],
            y: self.position[1],
        }
    }
    pub fn as_pos(&self) -> Pos2 {
        Pos2 {
            x: self.position[0],
            y: self.position[1],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Shape {
    pub(crate) vertices: Vec<Vertex>,
}

impl Shape {
    pub fn new_rectangle(aspect_ratio: f32) -> Self {
        let mut vertices = Vec::new();
        let mut sign_x = -1.0;
        let mut sign_y = 1.0;
        for numbers in 1..6 {
            vertices.push(Vertex {
                position: [sign_x * aspect_ratio, sign_y],
                tex_coords: [sign_x / 2.0 + 0.5, sign_y / 2.0 + 0.5],
            });
            if numbers == 3 {
                vertices.push(Vertex {
                    position: [sign_x * aspect_ratio, sign_y],
                    tex_coords: [sign_x / 2.0 + 0.5, sign_y / 2.0 + 0.5],
                });
            }
            if numbers % 2 == 1 {
                sign_y *= -1.0;
            } else {
                sign_x *= -1.0;
            }
        }

        Shape { vertices }
    }
}

pub struct Dis {
    pub(crate) data: Option<TextureData>,
}

#[tokio::main]
async fn main() {
    let cli = Client::new();

    let mut disp = Dis { data: None };
    let event_loop = glutin::event_loop::EventLoopBuilder::with_user_event().build();
    let display = create_display(&event_loop);

    let mut egui_glium = egui_glium::EguiGlium::new(&display, &event_loop);

    let vertex_shader_src = r#"
    #version 140
    in vec2 position;
    in vec2 tex_coords;
    out vec2 v_tex_coords;

    uniform float aspect;
    uniform float zoom;
    uniform vec2 offset;

    void main() {
        v_tex_coords = tex_coords;
        gl_Position = vec4((position.x+offset.x)*zoom, (position.y*aspect+offset.y)*zoom, 0.0, 1.0);
    }
    "#;

    let fragment_shader_src = r#"
    #version 140
    in vec2 v_tex_coords;
    out vec4 color;

    uniform sampler2D tex;

    void main() {
        //color = vec4(1.0, 0.0, 0.0, 1.0);
        color = texture(tex, v_tex_coords);
    }
    "#;

    let program =
        glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
            .unwrap();

    //let image = texture_manager::get_texture(&display, &egui_glium.egui_ctx);

    let mut input = String::new();

    let mut selected_index: u32 = 0;

    let THEPATH: String = inner_main()
        .unwrap()
        .as_path()
        .as_os_str()
        .to_str()
        .unwrap()
        .to_string()
        + "\\images\\";

    let paths = fs::read_dir(&THEPATH).unwrap();

    let mut all_images: Vec<String> = Vec::new();
    for path in paths {
        let path = path.unwrap().path().display().to_string();
        let split = path.split("\\");
        let vec = split.collect::<Vec<&str>>();

        let result = vec.get(vec.len() - 1).unwrap().to_owned().to_owned();
        println!("Name: {}", &result);
        if result.contains(".png") {
            all_images.push(result);
        }
    }

    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            let mut quit = false;

            let repaint_after = egui_glium.run(&display, |egui_ctx| {
                // let run_results = gui::run(egui_ctx, &display, &input_info, gui_info, &mut world_info);
                let mut main_panel = egui::Window::new("provide name");
                main_panel = main_panel.resizable(false);
                egui::Window::show(main_panel, egui_ctx, |ui| {
                    let mut changed = false;

                    let label = egui::Label::new(
                        "Currently selected = ".to_owned() + &selected_index.to_string(),
                    );
                    ui.add(label);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        if ui.add(egui::Button::new("<")).clicked() {
                            if selected_index > 0 {
                                selected_index -= 1;
                            }
                            changed = true;
                        }
                        if ui.add(egui::Button::new(">")).clicked() {
                            if (selected_index as i32) < (all_images.len() as i32)-1 {
                                selected_index += 1;
                            }
                            changed = true;
                        }
                    });

                    if changed {
                        let fuckrust = all_images.get(selected_index as usize);
                        let image = texture_manager::get_dynamic_image(
                            &(THEPATH.clone() + fuckrust.unwrap()),
                        );
                        if image.is_some() {
                            let load_data = texture_manager::get_texture_data(
                                &display,
                                &egui_ctx,
                                &image.unwrap(),
                            );
                            disp.data = Some(load_data);
                        } else {
                            disp.data = None;
                        }
                    }

                    if ui.button("quit").clicked() {
                        quit = true;
                    }
                    //ui.add(egui::Slider::new(&mut input.zoom_modifier, 0.01..=0.05).text("Zoom Speed"));
                });
                // quit = run_results.0;
                // gui_info = run_results.1;
                //egui_ctx.load_texture(name, image, filter);
            });

            *control_flow = if quit {
                glutin::event_loop::ControlFlow::Exit
            } else if repaint_after.is_zero() {
                display.gl_window().window().request_redraw();
                glutin::event_loop::ControlFlow::Poll
            } else if let Some(repaint_after_instant) =
                std::time::Instant::now().checked_add(repaint_after)
            {
                glutin::event_loop::ControlFlow::WaitUntil(repaint_after_instant)
            } else {
                glutin::event_loop::ControlFlow::Wait
            };

            {
                //vertex_info = info::collect_vertex_shader_info(vertex_info, &input_info, &display, &egui_glium);

                use glium::Surface as _;
                let mut target = display.draw();

                let color = egui::Rgba::from_rgb(0.350, 0.109, 0.0490);
                target.clear_color(color[0], color[1], color[2], color[3]);

                // draw things behind egui here
                //let mut target = draw_things(&display, target, &program, &image, &vertex_info);
                let mut target = draw_things(&display, target, &program, &mut disp);
                //let draw_pass = data_displayer::draw_things(&display, target, &program, &vertex_info, &world_info);
                //let mut target = draw_pass;

                egui_glium.paint(&display, &mut target);

                // draw things on top of egui here

                target.finish().unwrap();
            }
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            glutin::event::Event::WindowEvent { event, .. } => {
                // match event {
                //     glutin::event::WindowEvent::CloseRequested => {
                //         println!("Received termination signal.");
                //         *control_flow = glutin::event_loop::ControlFlow::Exit;
                //         return;
                //     }
                //     /* The code to get the mouse position (And print it to the console) */
                //     // glutin::event::WindowEvent::CursorMoved { position, .. } => {
                //     //     //println!("Mouse position: {:?}x{:?}", position.x as u16, position.y as u16);
                //     //     input_info.mouse_pos.0 = position.x as f32;
                //     //     input_info.mouse_pos.1 = position.y as f32;
                //     // }
                //     // // _ => return,
                //     // glutin::event::WindowEvent::MouseWheel { delta, .. } => {
                //     //     if let MouseScrollDelta::LineDelta(_, y) = delta {
                //     //         scroll = true;
                //     //         input_info.scroll_delta = y;
                //     //     }
                //     // }

                //     // glutin::event::WindowEvent::MouseInput { button, state, .. } => {
                //     //     if let MouseButton::Left = button {
                //     //         if let ElementState::Pressed = state {
                //     //             if input_info.left_mouse == false {
                //     //                 input_info.drag_start = input_info.mouse_pos;
                //     //                 vertex_info.init_camera[0] = vertex_info.camera[0];
                //     //                 vertex_info.init_camera[1] = vertex_info.camera[1];
                //     //             }

                //     //             // vertex_info.init_offset[0] = vertex_info.offset[0];
                //     //             // vertex_info.init_offset[1] = vertex_info.offset[1];

                //     //             input_info.left_mouse = true;
                //     //         } else {
                //     //             input_info.left_mouse = false;
                //     //         }
                //     //     }
                //     // }

                //     // glutin::event::WindowEvent::ModifiersChanged(state) => {
                //     //     if state.ctrl() {
                //     //         input_info.control = true;
                //     //     } else {
                //     //         input_info.control = false;
                //     //     }
                //     // }

                //     // _ => {
                //     //     input_info.scroll_delta = 0.0;
                //     //     return;
                //     // }

                //     // glutin::event::WindowEvent::KeyboardInput { input , ..} => {
                //     //     if (input.virtual_keycode.is_some()) {
                //     //         let a = input.virtual_keycode.unwrap();

                //     //     }
                //     // }

                //     //_ => {return;}
                // }

                // important piece of egui code that allows egui to update
                let event_response = egui_glium.on_event(&event);

                if event_response {
                    display.gl_window().window().request_redraw();
                }
            }

            glutin::event::Event::NewEvents(glutin::event::StartCause::ResumeTimeReached {
                ..
            }) => {
                display.gl_window().window().request_redraw();
            }
            _ => (),
        }
    });
}

fn create_display(event_loop: &glutin::event_loop::EventLoop<()>) -> glium::Display {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize {
            width: 800.0,
            height: 600.0,
        })
        .with_title("World Builder");

    let context_builder = glutin::ContextBuilder::new()
        .with_depth_buffer(0)
        .with_srgb(true)
        .with_stencil_buffer(0)
        .with_vsync(true);

    glium::Display::new(window_builder, context_builder, event_loop).unwrap()
}

pub fn draw_things(
    dis: &Display,
    mut target: Frame,
    pro: &Program,
    display_settings: &Dis,
) -> Frame {
    let dimensions = dis.get_framebuffer_dimensions();
    let view_ratio = (dimensions.0 as f32) / (dimensions.1 as f32);

    //let texture = glium::texture::SrgbTexture2d::new(dis, img).unwrap();
    //world_info.world_texture;
    if display_settings.data.is_some() {
        let asp = display_settings
            .data
            .as_ref()
            .unwrap()
            .gui_texture
            .aspect_ratio();
        let shape = Shape::new_rectangle(asp);
        //println!("{:?}",shape);

        let shape = shape.vertices;
        //let shape = &world_info.triangles;

        let vertex_buffer = glium::VertexBuffer::new(dis, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        let texture = &display_settings.data.as_ref().unwrap().vertex_texture;

        let uniforms = uniform! {tex: texture, aspect: view_ratio, zoom: 0.465 as f32, offset: [0.0 as f32,0.0 as f32]};

        // &glium::uniforms::EmptyUniforms

        let params = glium::DrawParameters {
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        target
            .draw(&vertex_buffer, &indices, &pro, &uniforms, &params)
            .unwrap();
    }

    return target;
}

fn inner_main() -> io::Result<PathBuf> {
    let mut dir = env::current_exe()?;
    dir.pop();
    // dir.push("Config");
    // dir.push("test.txt");
    Ok(dir)
}

// pub async fn get_image_from_url() -> reqwest::Result<()> {
//     let body = reqwest::get("https://www.rust-lang.org/static/images/rust-social-wide.jpg").await?;
//     // let bytes = body.bytes().await?;
//     // let image = image::load_from_memory(&bytes);

//     // println!("image ok is {}", image.is_ok());
//     return Ok(());

// }
// pub async fn test() {
//     println!("test");
// }

pub async fn load_image_from_url(url: &str, cli: &Client) -> Option<DynamicImage> {
    match cli.get(url).send().await {
        Ok(mut response) => {
            // check if 200 ok
            if response.status() == reqwest::StatusCode::OK {
                match response.bytes().await {
                    Ok(bytes) => match image::load_from_memory(&bytes) {
                        Ok(image) => return Some(image),
                        Err(_) => {
                            println!("couldn't load image from bytes");
                            return None;
                        }
                    },
                    Err(_) => {
                        println!("Could not read bytes");
                        return None;
                    }
                }
            } else {
                println!("response was not 200 OK.");
                return None;
            }
        }
        Err(_) => {
            println!("couldn't make the request");
            return None;
        }
    }
}

pub async fn load_page(name: &str, cli: &Client) -> Option<String> {
    let mut con = ("https://forgottenrealms.fandom.com/wiki/").to_owned();
    con.push_str(name);
    //println!("{}", con.as_str());
    match cli.get(con.as_str()).send().await {
        Ok(mut response) => {
            // check if 200 ok
            if response.status() == reqwest::StatusCode::OK {
                match response.text().await {
                    Ok(text) => {
                        return Some(text);
                    }
                    Err(_) => {
                        println!("Could not read text");
                        return None;
                    }
                }
            } else {
                println!("response was not 200 OK.");
                return None;
            }
        }
        Err(_) => {
            println!("couldn't make the request");
            return None;
        }
    }
}
