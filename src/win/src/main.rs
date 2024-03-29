use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::*;

use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::mem::ManuallyDrop;

use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{Direct2D::Common::*, Direct2D::*, Direct3D::*, Direct3D11::*, Dxgi::Common::*, Dxgi::*, Gdi::*},
        System::{Com::*, LibraryLoader::*, Performance::*},
        UI::{Controls::Dialogs::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
use y_nes::nes::*;

const COLORS: [[u8; 3]; 64] = [
    [84, 84, 84],
    [0, 30, 116],
    [8, 16, 144],
    [48, 0, 136],
    [68, 0, 100],
    [92, 0, 48],
    [84, 4, 0],
    [60, 24, 0],
    [32, 42, 0],
    [8, 58, 0],
    [0, 64, 0],
    [0, 60, 0],
    [0, 50, 60],
    [0, 0, 0],
    [0, 0, 0],
    [0, 0, 0],
    [152, 150, 152],
    [8, 76, 196],
    [48, 50, 236],
    [92, 30, 228],
    [136, 20, 176],
    [160, 20, 100],
    [152, 34, 32],
    [120, 60, 0],
    [84, 90, 0],
    [40, 114, 0],
    [8, 124, 0],
    [0, 118, 40],
    [0, 102, 120],
    [0, 0, 0],
    [0, 0, 0],
    [0, 0, 0],
    [236, 238, 236],
    [76, 154, 236],
    [120, 124, 236],
    [176, 98, 236],
    [228, 84, 236],
    [236, 88, 180],
    [236, 106, 100],
    [212, 136, 32],
    [160, 170, 0],
    [116, 196, 0],
    [76, 208, 32],
    [56, 204, 108],
    [56, 180, 204],
    [60, 60, 60],
    [0, 0, 0],
    [0, 0, 0],
    [236, 238, 236],
    [168, 204, 236],
    [188, 188, 236],
    [212, 178, 236],
    [236, 174, 236],
    [236, 174, 212],
    [236, 180, 176],
    [228, 196, 144],
    [204, 210, 120],
    [180, 222, 120],
    [168, 226, 144],
    [152, 226, 180],
    [160, 214, 228],
    [160, 162, 160],
    [0, 0, 0],
    [0, 0, 0],
];

const WINDOW_TITLE: PCSTR = PCSTR(b"yNES for Windows\0".as_ptr() as _);
const WINDOW_TITLE_OVERLOAD: PCSTR = PCSTR(b"yNES for Windows - [overload!]\0".as_ptr() as _);

fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;
    }
    let mut window = Window::new()?;
    window.run()
}

struct Window {
    handle: HWND,
    factory: ID2D1Factory1,
    dxfactory: IDXGIFactory2,

    target: Option<ID2D1DeviceContext>,
    swapchain: Option<IDXGISwapChain1>,
    canvas: Option<ID2D1Bitmap1>,
    frame_buffer: Box<[u8; 256 * 240 * 4]>,
    dpi: f32,
    visible: bool,
    occlusion: u32,
    frequency: i64,
    nes: Option<Nes>,
    test_audio_out: Option<std::fs::File>,
    test_audio_count: u8,
    target_fps: u16,
    start_time: i64,
    rendered_frames: u64,
    pcm_buffers: Box<[f32; 3025]>,
    sdl_context: Sdl,
    audio_subsystem: AudioSubsystem,
    audio_queue: AudioQueue<f32>,
}

impl Window {
    fn new() -> Result<Self> {
        let factory = create_factory()?;
        let dxfactory: IDXGIFactory2 = unsafe { CreateDXGIFactory1()? };

        let mut dpi = 0.0;
        let mut dpiy = 0.0;
        unsafe { factory.GetDesktopDpi(&mut dpi, &mut dpiy) };

        let mut frequency = 0;
        unsafe { QueryPerformanceFrequency(&mut frequency).ok()? };

        let sdl_context = sdl2::init().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();
        let desired_spec = AudioSpecDesired { freq: Some(178680), channels: Some(1), samples: Some(3025) };
        let device = audio_subsystem.open_queue::<f32, _>(None, &desired_spec).unwrap();

        Ok(Window {
            handle: HWND(0),
            factory,
            dxfactory,

            target: None,
            swapchain: None,
            canvas: None,
            frame_buffer: Box::new([0; 256 * 240 * 4]),
            dpi,
            visible: false,
            occlusion: 0,
            frequency,
            nes: None,
            test_audio_out: None,
            test_audio_count: 0,
            target_fps: 60,
            start_time: 0,
            rendered_frames: 0,
            pcm_buffers: Box::new([0.0; 3025]),
            sdl_context: sdl_context,
            audio_subsystem: audio_subsystem,
            audio_queue: device,
        })
    }

    fn run(&mut self) -> Result<()> {
        unsafe {
            let instance = GetModuleHandleA(None).unwrap();

            let wc = WNDCLASSA {
                hInstance: instance,
                lpszClassName: PCSTR(b"window\0".as_ptr() as _),
                lpszMenuName: PCSTR(b"menu\0".as_ptr() as _),
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(Self::wndproc),
                ..Default::default()
            };

            let atom = RegisterClassA(&wc);
            debug_assert!(atom != 0);

            let handle = CreateWindowExA(
                Default::default(),
                PCSTR(b"window\0".as_ptr() as _),
                WINDOW_TITLE,
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                instance,
                Some(self as *mut _ as _),
            );

            self.set_window_size(2);

            debug_assert!(handle != HWND(0));
            debug_assert!(handle == self.handle);
            let mut message = MSG::default();

            self.audio_queue.resume();

            loop {
                if self.visible {
                    self.render()?;

                    while PeekMessageA(&mut message, None, 0, 0, PM_REMOVE).into() {
                        if message.message == WM_QUIT {
                            return Ok(());
                        }
                        DispatchMessageA(&message);
                    }
                } else {
                    GetMessageA(&mut message, None, 0, 0);

                    if message.message == WM_QUIT {
                        return Ok(());
                    }

                    DispatchMessageA(&message);
                }
            }
        }
    }

    fn render(&mut self) -> Result<()> {
        if self.target.is_none() {
            let device = create_device()?;
            let target = create_render_target(&self.factory, &device)?;
            unsafe { target.SetDpi(self.dpi, self.dpi) };

            let swapchain = create_swapchain(&device, self.handle)?;
            create_swapchain_bitmap(&swapchain, &target)?;

            self.target = Some(target);
            self.swapchain = Some(swapchain);
            self.create_device_size_resources()?;
        }

        if self.nes.is_some() {
            let nes = self.nes.as_mut().unwrap();
            let mut input = PadInput { ..Default::default() };
            unsafe {
                input = PadInput {
                    a: GetKeyState('X' as i32) < 0,
                    b: GetKeyState('Z' as i32) < 0,
                    select: GetKeyState(VK_ESCAPE.0.into()) < 0,
                    start: GetKeyState(VK_RETURN.0.into()) < 0,
                    up: GetKeyState(VK_UP.0.into()) < 0,
                    down: GetKeyState(VK_DOWN.0.into()) < 0,
                    left: GetKeyState(VK_LEFT.0.into()) < 0,
                    right: GetKeyState(VK_RIGHT.0.into()) < 0,
                };
            }
            let inputs = PadInputs { pad1: input, pad2: Default::default() };
            let current_time = get_time().unwrap();
            let time_diff = current_time - self.start_time;
            let time_diff_sec = (time_diff as f64) / (self.frequency as f64);
            let current_frames = (time_diff_sec * self.target_fps as f64) as u64;
            let rendered_frames = self.rendered_frames;
            let need_render_frames = current_frames - rendered_frames;
            let overload_frames = need_render_frames.saturating_sub(5);
            unsafe {
                if overload_frames > 0 {
                    SetWindowTextA(self.handle, WINDOW_TITLE_OVERLOAD);
                } else {
                    SetWindowTextA(self.handle, WINDOW_TITLE);
                }
            }

            if self.target_fps != 60 || self.audio_queue.size() <= 2978 * 4 * 3 {
                let mut pcm_filled: usize = 0;
                for _ in 0..std::cmp::min(need_render_frames, 5) {
                    let mut end_frame = false;
                    while end_frame != true {
                        let result = nes.clock(&inputs);
                        end_frame = result.0;
                        let apu_out = result.1;
                        if apu_out.is_some() {
                            //とりあえず雑に間引く
                            if self.test_audio_count % 10 == 0 {
                                let pcm = apu_out.unwrap();
                                if pcm_filled < 3025 {
                                    self.pcm_buffers[pcm_filled] = pcm;
                                    pcm_filled += 1;
                                }
                                if self.test_audio_out.is_some() {
                                    let file = self.test_audio_out.as_mut().unwrap();
                                    let pcm_bytes = pcm.to_be_bytes();
                                    file.write_all(&pcm_bytes);
                                }
                            }
                            self.test_audio_count += 1;
                            self.test_audio_count %= 10;
                        }
                    }
                }
                if pcm_filled > 0 && self.audio_queue.size() <= 2978 * 4 * 3 {
                    self.audio_queue.queue_audio(&self.pcm_buffers[0..pcm_filled]);
                }
                self.rendered_frames += need_render_frames;

                let screen = nes.get_screen();
                for (index, pixel) in screen.iter().enumerate() {
                    let index = index * 4;
                    let color = COLORS[*pixel as usize];
                    self.frame_buffer[index] = color[2]; //B
                    self.frame_buffer[index + 1] = color[1]; //G
                    self.frame_buffer[index + 2] = color[0]; //R
                    self.frame_buffer[index + 3] = 0xFF; //A
                }
            }

            let target = self.target.as_ref().unwrap();
            unsafe { target.BeginDraw() };
            self.draw()?;
            unsafe {
                let target = self.target.as_ref().unwrap();
                target.EndDraw(None, None)?;
            }
        }

        if let Err(error) = self.present(1, 0) {
            if error.code() == DXGI_STATUS_OCCLUDED {
                self.occlusion = unsafe { self.dxfactory.RegisterOcclusionStatusWindow(self.handle, WM_USER)? };
                self.visible = false;
            } else {
                self.release_device();
            }
        }

        Ok(())
    }

    fn release_device(&mut self) {
        self.target = None;
        self.swapchain = None;
        self.release_device_resources();
    }

    fn release_device_resources(&mut self) {
        self.canvas = None;
    }

    fn present(&self, sync: u32, flags: u32) -> Result<()> {
        unsafe { self.swapchain.as_ref().unwrap().Present(sync, flags).ok() }
    }

    fn draw(&mut self) -> Result<()> {
        unsafe {
            {
                let target = self.target.as_ref().unwrap();
                target.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }));
            }

            self.draw_canvas()?;

            let canvas = self.canvas.as_ref().unwrap();
            let target = self.target.as_ref().unwrap();
            let size = target.GetSize();
            let target_aspect = size.width as f32 / size.height as f32;
            let original_width = 256;
            let original_height = 240;
            let original_aspect = original_width as f32 / original_height as f32;
            let (width, height) = if target_aspect > original_aspect {
                (size.height * original_aspect, size.height)
            } else {
                (size.width, size.width / original_aspect)
            };

            let rect = D2D_RECT_F {
                left: (size.width - width) / 2.0,
                top: (size.height - height) / 2.0,
                right: (size.width - width) / 2.0 + width,
                bottom: (size.height - height) / 2.0 + height,
            };
            target.DrawBitmap(
                canvas,
                Some(&rect),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
                None,
            );
        }

        Ok(())
    }

    fn draw_canvas(&mut self) -> Result<()> {
        let canvas = self.canvas.as_ref().unwrap();
        unsafe { canvas.CopyFromMemory(None, self.frame_buffer.as_ptr() as *const std::ffi::c_void, 256 * 4) }
    }

    fn create_device_size_resources(&mut self) -> Result<()> {
        let target = self.target.as_ref().unwrap();
        let canvas = self.create_canvas(target)?;
        self.canvas = Some(canvas);

        Ok(())
    }

    fn create_canvas(&self, target: &ID2D1DeviceContext) -> Result<ID2D1Bitmap1> {
        let size = D2D_SIZE_U { width: 256, height: 240 };

        let properties = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_IGNORE },
            dpiX: self.dpi,
            dpiY: self.dpi,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET,
            colorContext: ManuallyDrop::new(None),
        };

        unsafe { target.CreateBitmap2(size, None, 256 * 4, &properties) }
    }

    fn resize_swapchain_bitmap(&mut self) -> Result<()> {
        if let Some(target) = &self.target {
            let swapchain = self.swapchain.as_ref().unwrap();
            unsafe { target.SetTarget(None) };

            if unsafe { swapchain.ResizeBuffers(0, 0, 0, DXGI_FORMAT_UNKNOWN, 0).is_ok() } {
                create_swapchain_bitmap(swapchain, &target)?;
                self.create_device_size_resources()?;
            } else {
                self.release_device();
            }

            self.render()?;
        }

        Ok(())
    }

    fn message_handler(&mut self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match message {
                WM_PAINT => {
                    let mut ps = PAINTSTRUCT::default();
                    BeginPaint(self.handle, &mut ps);
                    self.render().unwrap();
                    EndPaint(self.handle, &ps);
                    LRESULT(0)
                }
                WM_SIZE => {
                    if wparam.0 != SIZE_MINIMIZED as usize {
                        self.resize_swapchain_bitmap().unwrap();
                    }
                    LRESULT(0)
                }
                WM_DISPLAYCHANGE => {
                    self.render().unwrap();
                    LRESULT(0)
                }
                WM_USER => {
                    if self.present(0, DXGI_PRESENT_TEST).is_ok() {
                        self.dxfactory.UnregisterOcclusionStatus(self.occlusion);
                        self.occlusion = 0;
                        self.visible = true;
                    }
                    LRESULT(0)
                }
                WM_ACTIVATE => {
                    self.visible = true; // TODO: unpack !HIWORD(wparam);
                    LRESULT(0)
                }
                WM_DESTROY => {
                    PostQuitMessage(0);
                    LRESULT(0)
                }
                WM_COMMAND => match wparam.0 as u16 {
                    100 => {
                        let mut buffer: [u8; 1024] = [0; 1024];
                        let mut file = OPENFILENAMEA {
                            lStructSize: std::mem::size_of::<OPENFILENAMEA>() as _,
                            hwndOwner: self.handle,
                            lpstrFilter: PCSTR(b"iNES file (*.nes)\0*.nes\0\0".as_ptr() as _),
                            lpstrFile: PSTR(&mut buffer[0]),
                            nMaxFile: 1024,
                            ..Default::default()
                        };
                        GetOpenFileNameA(&mut file);
                        let file_path = std::ffi::CStr::from_ptr(buffer.as_ptr() as _).to_str().unwrap();
                        if file_path != "" {
                            println!("File selected: {}", file_path);
                            let file = File::open(file_path);
                            if file.is_err() {
                                println!("open file error");
                                return LRESULT(0);
                            }
                            let mut contents = vec![];
                            if file.unwrap().read_to_end(&mut contents).is_err() {
                                println!("read file error");
                                return LRESULT(0);
                            }
                            self.nes = Some(Nes::new(contents.as_slice()).unwrap());
                        }
                        self.start_time = get_time().unwrap();
                        self.rendered_frames = 0;
                        LRESULT(0)
                    }
                    param @ 200..=299 => {
                        match param {
                            200 => self.set_window_size(1),
                            201 => self.set_window_size(2),
                            202 => self.set_window_size(3),
                            203 => self.set_window_size(4),
                            204 => self.set_window_size(5),
                            _ => panic!(),
                        };
                        LRESULT(0)
                    }
                    param @ 300..=399 => {
                        match param {
                            300 => self.target_fps = 60,
                            301 => self.target_fps = 30,
                            302 => self.target_fps = 15,
                            310 => self.target_fps = 120,
                            311 => self.target_fps = 240,
                            _ => panic!(),
                        };
                        self.start_time = get_time().unwrap();
                        self.rendered_frames = 0;
                        LRESULT(0)
                    }
                    60000 => {
                        self.open_about_window();
                        LRESULT(0)
                    }
                    _ => DefWindowProcA(self.handle, message, wparam, lparam),
                },
                _ => DefWindowProcA(self.handle, message, wparam, lparam),
            }
        }
    }

    fn set_window_size(&self, multiply: i32) {
        let mut window_rect = RECT { ..Default::default() };
        let mut client_rect = RECT { ..Default::default() };
        unsafe {
            GetWindowRect(self.handle, &mut window_rect);
            GetClientRect(self.handle, &mut client_rect);
            let new_width =
                (256 * multiply) - (client_rect.right - client_rect.left) + (window_rect.right - window_rect.left);
            let new_height =
                (240 * multiply) - (client_rect.bottom - client_rect.top) + (window_rect.bottom - window_rect.top);
            SetWindowPos(
                self.handle,
                None,
                0,
                0,
                new_width,
                new_height,
                SWP_NOMOVE | SWP_NOZORDER,
            );
        }
    }

    extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            if message == WM_NCCREATE {
                let cs = lparam.0 as *const CREATESTRUCTA;
                let this = (*cs).lpCreateParams as *mut Self;
                (*this).handle = window;

                SetWindowLong(window, GWLP_USERDATA, this as _);
            } else {
                let this = GetWindowLong(window, GWLP_USERDATA) as *mut Self;

                if !this.is_null() {
                    return (*this).message_handler(message, wparam, lparam);
                }
            }

            DefWindowProcA(window, message, wparam, lparam)
        }
    }

    fn open_about_window(&self) {
        unsafe {
            MessageBoxA(
                self.handle,
                PCSTR(
                    format!(
                        "yNES for Windows by YDKK\n\nEmulator Core: v{}\nWindows Frontend: v{}\n\nhttps://github.com/YDKK/yNES\0",
                        Nes::get_version(),
                        env!("CARGO_PKG_VERSION")
                    )
                    .as_ptr() as _,
                ),
                PCSTR("yNES for Windows\0".as_ptr() as _),
                MB_OK,
            );
        }
    }
}

fn create_factory() -> Result<ID2D1Factory1> {
    let mut options = D2D1_FACTORY_OPTIONS::default();

    if cfg!(debug_assertions) {
        options.debugLevel = D2D1_DEBUG_LEVEL_INFORMATION;
    }

    unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, Some(&options)) }
}

fn create_device() -> Result<ID3D11Device> {
    let mut result = create_device_with_type(D3D_DRIVER_TYPE_HARDWARE);

    if let Err(err) = &result {
        if err.code() == DXGI_ERROR_UNSUPPORTED {
            result = create_device_with_type(D3D_DRIVER_TYPE_WARP);
        }
    }

    result
}

fn create_device_with_type(drive_type: D3D_DRIVER_TYPE) -> Result<ID3D11Device> {
    let mut flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;

    if cfg!(debug_assertions) {
        flags |= D3D11_CREATE_DEVICE_DEBUG;
    }

    let mut device = None;

    unsafe {
        D3D11CreateDevice(
            None,
            drive_type,
            None,
            flags,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )
        .map(|()| device.unwrap())
    }
}

fn create_render_target(factory: &ID2D1Factory1, device: &ID3D11Device) -> Result<ID2D1DeviceContext> {
    unsafe {
        let d2device = factory.CreateDevice(&device.cast::<IDXGIDevice>()?)?;

        let target = d2device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;

        target.SetUnitMode(D2D1_UNIT_MODE_DIPS);

        Ok(target)
    }
}

fn create_swapchain(device: &ID3D11Device, window: HWND) -> Result<IDXGISwapChain1> {
    let factory = get_dxgi_factory(device)?;

    let props = DXGI_SWAP_CHAIN_DESC1 {
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        ..Default::default()
    };

    unsafe { factory.CreateSwapChainForHwnd(device, window, &props, None, None) }
}

fn get_dxgi_factory(device: &ID3D11Device) -> Result<IDXGIFactory2> {
    let dxdevice = device.cast::<IDXGIDevice>()?;
    unsafe { dxdevice.GetAdapter()?.GetParent() }
}

fn create_swapchain_bitmap(swapchain: &IDXGISwapChain1, target: &ID2D1DeviceContext) -> Result<()> {
    let surface: IDXGISurface = unsafe { swapchain.GetBuffer(0)? };

    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_IGNORE },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        ..Default::default()
    };

    unsafe {
        let bitmap = target.CreateBitmapFromDxgiSurface(&surface, Some(&props))?;
        target.SetTarget(&bitmap);
    };

    Ok(())
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "32")]
unsafe fn SetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    SetWindowLongA(window, index, value as _) as _
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "64")]
unsafe fn SetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    SetWindowLongPtrA(window, index, value)
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "32")]
unsafe fn GetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    GetWindowLongA(window, index) as _
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "64")]
unsafe fn GetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    GetWindowLongPtrA(window, index)
}

fn get_time() -> Result<i64> {
    unsafe {
        let mut time = 0;
        QueryPerformanceCounter(&mut time).ok()?;
        Ok(time)
    }
}
