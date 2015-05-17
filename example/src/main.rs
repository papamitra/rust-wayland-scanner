#![feature(libc)]
#![feature(unique)]

extern crate wayland;

use wayland::client::wayland_client::*;
use wayland::client::wayland_client_protocol::*;

extern crate libc;
extern crate tempfile;

use std::ffi::CString;
use std::ffi::CStr;
use std::rc::Rc;
use std::option::Option;

#[derive(Clone)]
struct Display {
    display: *mut Struct_wl_display,
    compositor: *mut Struct_wl_compositor,
    shm: *mut Struct_wl_shm,
    shell: *mut Struct_wl_shell,
    formats: u32,
}

impl Drop for Display {
    fn drop(&mut self) {
        println!("call Display Drop");
        unsafe { wl_display_disconnect(self.display);}
    }
}

#[derive(Clone)]
struct Buffer {
    buffer: *mut Struct_wl_buffer,
    shm_data: Option<Rc<MemoryMap>>,
}

fn cstr_to_string(cstr: *const ::libc::c_char) -> String {
    let buf = unsafe { CStr::from_ptr(cstr).to_bytes() };
    String::from_utf8(buf.to_vec()).unwrap()
}

extern "C" fn shm_format(data: *mut libc::c_void, wl_shm: *mut Struct_wl_shm, format: u32) {
    let d: &mut Display = unsafe { &mut *(data as *mut Display)};

    if format < 31 {
        d.formats |= 1 << format;
    }
}

static WL_SHM_LISTENER: Struct_wl_shm_listener = Struct_wl_shm_listener {
    format: Some(shm_format),
};

extern "C" fn handle_ping(data: *mut libc::c_void, shell_surface: *mut Struct_wl_shell_surface, serial: u32) {
    wl_shell_surface_pong(shell_surface, serial);
}

extern "C" fn handle_configure(data: *mut libc::c_void, shell_surface: *mut Struct_wl_shell_surface,
                               edges: u32, width: i32, height: i32) {
}

extern "C" fn handle_popup_done(data: *mut libc::c_void, shell_surface: *mut Struct_wl_shell_surface) {
}

static SHELL_SURFACE_LISTENER: Struct_wl_shell_surface_listener = Struct_wl_shell_surface_listener {
    ping: Some(handle_ping),
    configure: Some(handle_configure),
    popup_done: Some(handle_popup_done)
};

extern "C" fn registry_handle_global(data: *mut ::libc::c_void, registry: *mut Struct_wl_registry,
                                     id: u32, interface: *const ::libc::c_char, version: u32) {

    let d: &mut Display = unsafe { &mut *(data as *mut Display)};

    match cstr_to_string(interface).as_ref() {
        "wl_compositor" => d.compositor =
            unsafe { &mut *(wl_registry_bind(registry, id, &wl_compositor_interface, 1) as *mut Struct_wl_compositor)},
        "wl_shm" => {
            d.shm = unsafe {&mut *(wl_registry_bind(registry, id, &wl_shm_interface, 1) as *mut Struct_wl_shm)};
            unsafe { wl_shm_add_listener(d.shm, &WL_SHM_LISTENER, std::mem::transmute(d))};
        },
        "wl_shell" => {
            d.shell = unsafe {&mut *(wl_registry_bind(registry, id, &wl_shell_interface, 1) as *mut Struct_wl_shell)};
        }
        _ => (),
    }
}

static WL_REGISTRY_LISTENER: Struct_wl_registry_listener = Struct_wl_registry_listener{
    global:Some(registry_handle_global),
    global_remove:None
};

#[derive(Clone)]
struct Surface {
    surface: *mut Struct_wl_surface,
}

#[derive(Clone)]
struct ShellSurface{
    ptr: *mut Struct_wl_shell_surface,
}

struct Window {
    width: i32,
    height: i32,
    display: Display,
    surface: Surface,
    buffer: Buffer,
    shell_surface: ShellSurface,
}

impl Window {
    fn new (display: &Display, width: i32, height: i32) -> Window {
        let surface = wl_compositor_create_surface(display.compositor);

        let shell_surface = wl_shell_get_shell_surface(display.shell,
                                                       surface);

        let window = Window{width: width, height:height,
               display: display.clone(),
               surface: Surface{surface: surface},
               buffer: Buffer{buffer: std::ptr::null_mut(), shm_data: None},
               shell_surface: ShellSurface{ptr: shell_surface},
        };

        if !shell_surface.is_null() {
            wl_shell_surface_add_listener(shell_surface,
                                          &SHELL_SURFACE_LISTENER, std::ptr::null_mut());
        }

        wl_shell_surface_set_title(shell_surface, "simple-shm");

        wl_shell_surface_set_toplevel(shell_surface);
        window
    }

    fn next_buffer(&mut self) -> Buffer {
        if self.buffer.buffer.is_null() {
            create_shm_buffer(&mut self.display, &mut self.buffer,
                              self.width, self.height,
                              WL_SHM_FORMAT_XRGB8888);
        }

        self.buffer.clone()
    }
}

use std::fs::OpenOptions;

use std::os::unix::prelude::*;

extern crate mmap;
use mmap::MemoryMap;
use mmap::MapOption;

use std::io::Write;

extern "C" fn buffer_release(data: *mut libc::c_void, buffer: *mut Struct_wl_buffer) {
    //struct buffer *mybuf = data;
    //mybuf->busy = 0;
}

static BUFFER_LISTENER: Struct_wl_buffer_listener = Struct_wl_buffer_listener {
    release: Some(buffer_release),
};

fn create_shm_buffer(display: &mut Display,
                     buffer: &mut Buffer,
                     width: i32, height:i32, format: u32) {

    let stride = width*4;
    let size = stride * height;
    let mut tf = tempfile::TempFile::new().unwrap();

    tf.set_len(size as u64).unwrap();
    let fd = tf.as_raw_fd();
    let map_opts = &[
        MapOption::MapFd(fd),
        MapOption::MapReadable,
        MapOption::MapWritable,
        MapOption::MapNonStandardFlags(libc::consts::os::posix88::MAP_SHARED),
        ];
    let data = MemoryMap::new(size as usize, map_opts).unwrap();

    let pool = unsafe{ wl_shm_create_pool(display.shm, fd, size) };
    buffer.buffer = unsafe { wl_shm_pool_create_buffer(pool, 0,
                                                       width, height,
                                                       stride, format)};
    buffer.shm_data = Some(Rc::new(data));
    wl_buffer_add_listener(buffer.buffer, &BUFFER_LISTENER, std::ptr::null_mut());
    wl_shm_pool_destroy(pool);
}

fn redraw(window: &mut Window) {
    let buffer = window.next_buffer();
    wl_surface_attach(window.surface.surface, buffer.buffer, 0, 0);
    wl_surface_damage(window.surface.surface,
                      20, 20, window.width - 40, window.height - 40);

//    if (callback)
//        wl_callback_destroy(callback);

//    window->callback = wl_surface_frame(window->surface);
//    wl_callback_add_listener(window->callback, &frame_listener, window);
    wl_surface_commit(window.surface.surface);

}

fn main() {
    let mut display = Display{ display: unsafe {wl_display_connect(std::ptr::null())},
                               compositor: std::ptr::null_mut(),
                               shm: std::ptr::null_mut(),
                               formats: 0,
                               shell: std::ptr::null_mut(),
    };
    assert!(!display.display.is_null());

    let reg = wl_display_get_registry(display.display);
    assert!(!reg.is_null());

    unsafe {wl_registry_add_listener(reg, &WL_REGISTRY_LISTENER, std::mem::transmute(&display));}

    unsafe {wl_display_roundtrip(display.display);}
    unsafe {wl_display_roundtrip(display.display);}

    let mut window = Window::new(&display, 250, 250);

    unsafe { wl_surface_damage(window.surface.surface, 0,0, window.width, window.height); }

    redraw(&mut window);

    unsafe {wl_display_roundtrip(display.display);}
    loop{
        unsafe {wl_display_dispatch(display.display);}
    }
}
