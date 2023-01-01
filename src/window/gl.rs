//! OpenGL context creation etc.

use crate::image::Image;
use sdl2::video::GLProfile;

pub use touchHLE_gl_bindings::{gl21compat, gl32core, gles11};

pub enum GLVersion {
    /// OpenGL ES 1.1
    #[allow(dead_code)]
    GLES11,
    /// OpenGL 2.1 compatibility profile
    GL21Compat,
    /// OpenGL 3.2 core profile
    GL32Core,
}

pub struct GLContext {
    gl_ctx: sdl2::video::GLContext,
    version: GLVersion,
}

pub fn create_gl_context(
    video_ctx: &sdl2::VideoSubsystem,
    window: &sdl2::video::Window,
    version: GLVersion,
) -> GLContext {
    let attr = video_ctx.gl_attr();
    match version {
        GLVersion::GLES11 => {
            attr.set_context_version(1, 1);
            attr.set_context_profile(GLProfile::GLES);
        }
        GLVersion::GL21Compat => {
            attr.set_context_version(2, 1);
            attr.set_context_profile(GLProfile::Compatibility);
        }
        GLVersion::GL32Core => {
            attr.set_context_version(3, 2);
            attr.set_context_profile(GLProfile::Core);
        }
    }

    let gl_ctx = window.gl_create_context().unwrap();

    GLContext { gl_ctx, version }
}

pub fn make_gl_context_current(
    video_ctx: &sdl2::VideoSubsystem,
    window: &sdl2::video::Window,
    gl_ctx: &GLContext,
) {
    window.gl_make_current(&gl_ctx.gl_ctx).unwrap();
    match gl_ctx.version {
        GLVersion::GLES11 => gles11::load_with(|s| video_ctx.gl_get_proc_address(s) as *const _),
        GLVersion::GL21Compat => {
            gl21compat::load_with(|s| video_ctx.gl_get_proc_address(s) as *const _)
        }
        GLVersion::GL32Core => {
            gl32core::load_with(|s| video_ctx.gl_get_proc_address(s) as *const _)
        }
    }
}

pub unsafe fn display_image(image: &Image) {
    let src_pixels = image.pixels();
    let (width, height) = image.dimensions();

    use gl32core as gl;

    let mut texture = 0;
    gl::GenTextures(1, &mut texture);

    gl::BindTexture(gl::TEXTURE_2D, texture);

    gl::TexImage2D(
        gl::TEXTURE_2D,
        0,
        gl::RGBA as _,
        width as _,
        height as _,
        0,
        gl::RGBA,
        gl::UNSIGNED_BYTE,
        src_pixels.as_ptr() as *const _,
    );
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

    let vertex_shader_src = "
#version 100
attribute vec2 pos;
varying vec2 texCoord;
void main() {
gl_Position = vec4(pos * 2.0 - 1.0, 0.0, 1.0);
texCoord = vec2(pos.x, 1.0 - pos.y); // glTexImage2D loads upside-down
}
";
    let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
    gl::ShaderSource(
        vertex_shader,
        1,
        &(vertex_shader_src.as_ptr() as *const _),
        &(vertex_shader_src.len() as _),
    );
    gl::CompileShader(vertex_shader);

    let fragment_shader_src = "
#version 100
precision mediump float;
uniform sampler2D tex;
varying vec2 texCoord;
void main() {
gl_FragColor = texture2D(tex, texCoord);
}
";
    let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
    gl::ShaderSource(
        fragment_shader,
        1,
        &(fragment_shader_src.as_ptr() as *const _),
        &(fragment_shader_src.len() as _),
    );
    gl::CompileShader(fragment_shader);

    let shader_program = gl::CreateProgram();
    gl::AttachShader(shader_program, vertex_shader);
    gl::AttachShader(shader_program, fragment_shader);
    gl::LinkProgram(shader_program);
    gl::UseProgram(shader_program);

    let pos_attrib = gl::GetAttribLocation(shader_program, "pos\0".as_ptr() as *const _);
    let tex_uniform = gl::GetUniformLocation(shader_program, "tex\0".as_ptr() as *const _);

    let mut vertex_array = 0;
    gl::GenVertexArrays(1, &mut vertex_array);
    gl::BindVertexArray(vertex_array);
    let vertices: [f32; 12] = [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    let mut vertex_buffer = 0;
    gl::GenBuffers(1, &mut vertex_buffer);
    gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
    gl::BufferData(
        gl::ARRAY_BUFFER,
        12 * 4,
        vertices.as_ptr() as *const _,
        gl::STATIC_DRAW,
    );
    gl::EnableVertexAttribArray(pos_attrib as _);
    gl::VertexAttribPointer(
        pos_attrib as _,
        2,
        gl::FLOAT,
        gl::FALSE,
        2 * 4,
        std::ptr::null(),
    );

    gl::ActiveTexture(gl::TEXTURE0);
    gl::Uniform1i(tex_uniform, 0);

    gl::DrawArrays(gl::TRIANGLES, 0, 6);

    gl::DeleteTextures(1, &texture);
    gl::DeleteShader(vertex_shader);
    gl::DeleteShader(fragment_shader);
    gl::DeleteProgram(shader_program);
    gl::DeleteVertexArrays(1, &vertex_array);
    gl::DeleteBuffers(1, &vertex_buffer);

    assert!(gl::GetError() == 0);
}
