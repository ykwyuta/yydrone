//! Direct3D11（図形・地形）と Direct2D/DirectWrite（文字）による描画層

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::Fxc::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::WindowsAndMessaging::*;
use yydrone_game::gfx::{Col, Font, Gfx};

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    c: Col,
}

const SHADER: &str = r#"
cbuffer CB : register(b0) { float4 xform; };
struct VSIn { float2 pos : POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
struct PSIn { float4 pos : SV_POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
PSIn vs_main(VSIn i) {
    PSIn o;
    o.pos = float4(i.pos * xform.xy + xform.zw, 0.0, 1.0);
    o.uv = i.uv;
    o.col = i.col;
    return o;
}
Texture2D tex : register(t0);
SamplerState smp : register(s0);
float4 ps_color(PSIn i) : SV_TARGET { return i.col; }
float4 ps_tex(PSIn i) : SV_TARGET { return tex.Sample(smp, i.uv) * i.col; }
"#;

const MAX_VERTS: usize = 300_000;

struct Texture {
    tex: ID3D11Texture2D,
    srv: ID3D11ShaderResourceView,
}

pub struct WinGfx {
    device: ID3D11Device,
    ctx: ID3D11DeviceContext,
    swapchain: IDXGISwapChain,
    rtv: Option<ID3D11RenderTargetView>,
    vs: ID3D11VertexShader,
    ps_color: ID3D11PixelShader,
    ps_tex: ID3D11PixelShader,
    layout: ID3D11InputLayout,
    vb: ID3D11Buffer,
    cb: ID3D11Buffer,
    blend: ID3D11BlendState,
    samp_lin: ID3D11SamplerState,
    samp_pt: ID3D11SamplerState,
    raster: ID3D11RasterizerState,
    hwnd: HWND,
    pw: u32,
    ph: u32,
    width: f32,
    height: f32,
    pub ui: f32,
    user_scale: f32,
    saved_window: Option<(isize, RECT)>,
    verts: Vec<Vertex>,
    textures: Vec<Texture>,
    d2d_factory: ID2D1Factory,
    d2d: Option<ID2D1RenderTarget>,
    brush: Option<ID2D1SolidColorBrush>,
    text_open: bool,
    dwrite: IDWriteFactory,
    fonts: Vec<IDWriteTextFormat>,
    measure_cache: std::collections::HashMap<(String, u8, u32), f32>,
    layout_cache: std::collections::HashMap<(String, u8, u32, u32), IDWriteTextLayout>,
    slot_cache: std::collections::HashMap<u8, Option<String>>,
}

/// 記録の置き場所（%LOCALAPPDATA%\\yydrone）
fn save_dir() -> std::path::PathBuf {
    std::env::var_os("LOCALAPPDATA").map_or_else(|| std::path::PathBuf::from("."), std::path::PathBuf::from).join("yydrone")
}

fn blob_bytes(b: &ID3DBlob) -> &[u8] {
    unsafe { std::slice::from_raw_parts(b.GetBufferPointer() as *const u8, b.GetBufferSize()) }
}

fn compile(src: &str, entry: PCSTR, target: PCSTR) -> Result<ID3DBlob> {
    let mut code = None;
    let mut err = None;
    let r = unsafe { D3DCompile(src.as_ptr() as _, src.len(), PCSTR::null(), None, None, entry, target, D3DCOMPILE_OPTIMIZATION_LEVEL3, 0, &mut code, Some(&mut err)) };
    if let Err(e) = r {
        if let Some(err) = err {
            let msg = String::from_utf8_lossy(blob_bytes(&err)).to_string();
            panic!("shader compile error: {msg}");
        }
        return Err(e);
    }
    Ok(code.unwrap())
}

fn make_fonts(dwrite: &IDWriteFactory) -> Result<Vec<IDWriteTextFormat>> {
    // gfx::Font と同じ並び
    let specs: [(f32, bool, bool); 7] = [(12.0, false, false), (14.0, false, false), (14.0, true, false), (18.0, false, true), (20.0, true, true), (40.0, true, true), (30.0, true, true)];
    let mut fonts = Vec::new();
    for (size, center, bold) in specs {
        unsafe {
            let f = dwrite.CreateTextFormat(w!("Yu Gothic UI"), None, if bold { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL }, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, size, w!("ja-jp"))?;
            if center {
                f.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
                f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            }
            f.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
            fonts.push(f);
        }
    }
    Ok(fonts)
}

impl WinGfx {
    pub fn new(hwnd: HWND, width: u32, height: u32) -> Result<WinGfx> {
        unsafe {
            let desc = DXGI_SWAP_CHAIN_DESC {
                BufferDesc: DXGI_MODE_DESC { Width: width, Height: height, Format: DXGI_FORMAT_B8G8R8A8_UNORM, ..Default::default() },
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                OutputWindow: hwnd,
                Windowed: TRUE,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                Flags: 0,
            };
            let mut swapchain = None;
            let mut device = None;
            let mut ctx = None;
            let levels = [D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_10_0];
            let mut r = D3D11CreateDeviceAndSwapChain(None, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, Some(&levels), D3D11_SDK_VERSION, Some(&desc), Some(&mut swapchain), Some(&mut device), None, Some(&mut ctx));
            if r.is_err() {
                r = D3D11CreateDeviceAndSwapChain(None, D3D_DRIVER_TYPE_WARP, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, Some(&levels), D3D11_SDK_VERSION, Some(&desc), Some(&mut swapchain), Some(&mut device), None, Some(&mut ctx));
            }
            r?;
            let device: ID3D11Device = device.unwrap();
            let ctx: ID3D11DeviceContext = ctx.unwrap();
            let swapchain: IDXGISwapChain = swapchain.unwrap();

            let vsb = compile(SHADER, s!("vs_main"), s!("vs_4_0"))?;
            let psc = compile(SHADER, s!("ps_color"), s!("ps_4_0"))?;
            let pst = compile(SHADER, s!("ps_tex"), s!("ps_4_0"))?;
            let mut vs = None;
            device.CreateVertexShader(blob_bytes(&vsb), None, Some(&mut vs))?;
            let mut ps_color = None;
            device.CreatePixelShader(blob_bytes(&psc), None, Some(&mut ps_color))?;
            let mut ps_tex = None;
            device.CreatePixelShader(blob_bytes(&pst), None, Some(&mut ps_tex))?;
            let elem = |name: PCSTR, fmt: DXGI_FORMAT, off: u32| D3D11_INPUT_ELEMENT_DESC { SemanticName: name, SemanticIndex: 0, Format: fmt, InputSlot: 0, AlignedByteOffset: off, InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA, InstanceDataStepRate: 0 };
            let elems = [elem(s!("POSITION"), DXGI_FORMAT_R32G32_FLOAT, 0), elem(s!("TEXCOORD"), DXGI_FORMAT_R32G32_FLOAT, 8), elem(s!("COLOR"), DXGI_FORMAT_R32G32B32A32_FLOAT, 16)];
            let mut layout = None;
            device.CreateInputLayout(&elems, blob_bytes(&vsb), Some(&mut layout))?;

            let mut vb = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC { ByteWidth: (MAX_VERTS * std::mem::size_of::<Vertex>()) as u32, Usage: D3D11_USAGE_DYNAMIC, BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32, CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32, MiscFlags: 0, StructureByteStride: 0 },
                None,
                Some(&mut vb),
            )?;
            let mut cb = None;
            device.CreateBuffer(&D3D11_BUFFER_DESC { ByteWidth: 16, Usage: D3D11_USAGE_DYNAMIC, BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32, CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32, MiscFlags: 0, StructureByteStride: 0 }, None, Some(&mut cb))?;
            let mut bd = D3D11_BLEND_DESC::default();
            bd.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: TRUE,
                SrcBlend: D3D11_BLEND_SRC_ALPHA,
                DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
            };
            let mut blend = None;
            device.CreateBlendState(&bd, Some(&mut blend))?;
            let mk_samp = |f: D3D11_FILTER| -> Result<ID3D11SamplerState> {
                let sd = D3D11_SAMPLER_DESC { Filter: f, AddressU: D3D11_TEXTURE_ADDRESS_CLAMP, AddressV: D3D11_TEXTURE_ADDRESS_CLAMP, AddressW: D3D11_TEXTURE_ADDRESS_CLAMP, MipLODBias: 0.0, MaxAnisotropy: 1, ComparisonFunc: D3D11_COMPARISON_NEVER, BorderColor: [0.0; 4], MinLOD: 0.0, MaxLOD: f32::MAX };
                let mut s = None;
                device.CreateSamplerState(&sd, Some(&mut s))?;
                Ok(s.unwrap())
            };
            let samp_lin = mk_samp(D3D11_FILTER_MIN_MAG_MIP_LINEAR)?;
            let samp_pt = mk_samp(D3D11_FILTER_MIN_MAG_MIP_POINT)?;
            let rd = D3D11_RASTERIZER_DESC { FillMode: D3D11_FILL_SOLID, CullMode: D3D11_CULL_NONE, DepthClipEnable: TRUE, ..Default::default() };
            let mut raster = None;
            device.CreateRasterizerState(&rd, Some(&mut raster))?;

            let d2d_factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let fonts = make_fonts(&dwrite)?;

            let mut g = WinGfx {
                device,
                ctx,
                swapchain,
                rtv: None,
                vs: vs.unwrap(),
                ps_color: ps_color.unwrap(),
                ps_tex: ps_tex.unwrap(),
                layout: layout.unwrap(),
                vb: vb.unwrap(),
                cb: cb.unwrap(),
                blend: blend.unwrap(),
                samp_lin,
                samp_pt,
                raster: raster.unwrap(),
                hwnd,
                pw: width,
                ph: height,
                width: width as f32,
                height: height as f32,
                ui: 1.0,
                user_scale: 1.0,
                saved_window: None,
                verts: Vec::with_capacity(MAX_VERTS),
                textures: Vec::new(),
                d2d_factory,
                d2d: None,
                brush: None,
                text_open: false,
                dwrite,
                fonts,
                measure_cache: Default::default(),
                layout_cache: Default::default(),
                slot_cache: Default::default(),
            };
            g.create_targets()?;
            g.update_scale();
            Ok(g)
        }
    }

    fn create_targets(&mut self) -> Result<()> {
        unsafe {
            let bb: ID3D11Texture2D = self.swapchain.GetBuffer(0)?;
            let mut rtv = None;
            self.device.CreateRenderTargetView(&bb, None, Some(&mut rtv))?;
            self.rtv = rtv;
            let surf: IDXGISurface = self.swapchain.GetBuffer(0)?;
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                dpiX: 96.0,
                dpiY: 96.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let rt = self.d2d_factory.CreateDxgiSurfaceRenderTarget(&surf, &props)?;
            rt.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            let brush = rt.CreateSolidColorBrush(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None)?;
            self.d2d = Some(rt);
            self.brush = Some(brush);
        }
        Ok(())
    }

    /// 画面の高さ 900px を基準に UI を拡大する（最小 1280×680 が収まる範囲で）
    fn update_scale(&mut self) {
        let (pw, ph) = (self.pw as f32, self.ph as f32);
        let cap = (ph / 680.0).min(pw / 1280.0).max(0.3);
        self.ui = (ph / 900.0 * self.user_scale).clamp(0.3, cap);
        self.width = (pw / self.ui).round().max(1.0);
        self.height = (ph / self.ui).round().max(1.0);
    }

    fn upload_draw(&mut self) {
        if self.verts.is_empty() {
            return;
        }
        unsafe {
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            if self.ctx.Map(&self.vb, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut m)).is_ok() {
                std::ptr::copy_nonoverlapping(self.verts.as_ptr(), m.pData as *mut Vertex, self.verts.len());
                self.ctx.Unmap(&self.vb, 0);
                self.ctx.Draw(self.verts.len() as u32, 0);
            }
        }
        self.verts.clear();
    }

    /// バックバッファを BMP に保存（自動確認用）
    pub fn save_backbuffer(&self, path: &str) -> Result<()> {
        unsafe {
            let bb: ID3D11Texture2D = self.swapchain.GetBuffer(0)?;
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            bb.GetDesc(&mut desc);
            desc.Usage = D3D11_USAGE_STAGING;
            desc.BindFlags = 0;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            desc.MiscFlags = 0;
            let mut st = None;
            self.device.CreateTexture2D(&desc, None, Some(&mut st))?;
            let st = st.unwrap();
            self.ctx.CopyResource(&st, &bb);
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            self.ctx.Map(&st, 0, D3D11_MAP_READ, 0, Some(&mut m))?;
            let (w, h) = (desc.Width as usize, desc.Height as usize);
            let mut out = Vec::with_capacity(54 + w * h * 4);
            let size = (54 + w * h * 4) as u32;
            out.extend_from_slice(b"BM");
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&[0, 0, 0, 0]);
            out.extend_from_slice(&54u32.to_le_bytes());
            out.extend_from_slice(&40u32.to_le_bytes());
            out.extend_from_slice(&(w as i32).to_le_bytes());
            out.extend_from_slice(&(-(h as i32)).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&32u16.to_le_bytes());
            out.extend_from_slice(&[0u8; 24]);
            for y in 0..h {
                let row = std::slice::from_raw_parts((m.pData as *const u8).add(y * m.RowPitch as usize), w * 4);
                out.extend_from_slice(row);
            }
            self.ctx.Unmap(&st, 0);
            let _ = std::fs::write(path, out);
        }
        Ok(())
    }

    fn close_text(&mut self) {
        if self.text_open {
            self.text_open = false;
            if let Some(rt) = &self.d2d {
                unsafe {
                    if let Err(e) = rt.EndDraw(None, None) {
                        if e.code() == D2DERR_RECREATE_TARGET {
                            self.d2d = None;
                            let _ = self.create_targets();
                        }
                    }
                }
            }
        }
    }
}

impl Gfx for WinGfx {
    fn size(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    fn begin(&mut self, clear: Col) {
        unsafe {
            let Some(rtv) = &self.rtv else { return };
            self.ctx.OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
            self.ctx.ClearRenderTargetView(rtv, &clear);
            let vp = D3D11_VIEWPORT { TopLeftX: 0.0, TopLeftY: 0.0, Width: self.pw as f32, Height: self.ph as f32, MinDepth: 0.0, MaxDepth: 1.0 };
            self.ctx.RSSetViewports(Some(&[vp]));
            self.ctx.RSSetState(&self.raster);
            self.ctx.OMSetBlendState(&self.blend, None, 0xffffffff);
            self.ctx.IASetInputLayout(&self.layout);
            self.ctx.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            let stride = std::mem::size_of::<Vertex>() as u32;
            let offset = 0u32;
            self.ctx.IASetVertexBuffers(0, 1, Some(&Some(self.vb.clone())), Some(&stride), Some(&offset));
            self.ctx.VSSetShader(&self.vs, None);
            self.ctx.VSSetConstantBuffers(0, Some(&[Some(self.cb.clone())]));
        }
        self.verts.clear();
    }

    fn set_xform(&mut self, sx: f32, sy: f32, ox: f32, oy: f32) {
        self.flush();
        let x = [sx, sy, ox, oy];
        unsafe {
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            if self.ctx.Map(&self.cb, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut m)).is_ok() {
                std::ptr::copy_nonoverlapping(x.as_ptr(), m.pData as *mut f32, 4);
                self.ctx.Unmap(&self.cb, 0);
            }
        }
    }

    #[inline]
    fn tri(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), col: Col) {
        if self.verts.len() + 3 > MAX_VERTS {
            self.flush();
        }
        self.verts.push(Vertex { x: a.0, y: a.1, u: 0.0, v: 0.0, c: col });
        self.verts.push(Vertex { x: b.0, y: b.1, u: 0.0, v: 0.0, c: col });
        self.verts.push(Vertex { x: c.0, y: c.1, u: 0.0, v: 0.0, c: col });
    }

    fn flush(&mut self) {
        if self.verts.is_empty() {
            return;
        }
        unsafe {
            self.ctx.PSSetShader(&self.ps_color, None);
        }
        self.upload_draw();
    }

    fn create_texture(&mut self, w: u32, h: u32, data: &[u32]) -> u32 {
        unsafe {
            let desc = D3D11_TEXTURE2D_DESC { Width: w, Height: h, MipLevels: 1, ArraySize: 1, Format: DXGI_FORMAT_R8G8B8A8_UNORM, SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 }, Usage: D3D11_USAGE_DEFAULT, BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32, CPUAccessFlags: 0, MiscFlags: 0 };
            let init = D3D11_SUBRESOURCE_DATA { pSysMem: data.as_ptr() as _, SysMemPitch: w * 4, SysMemSlicePitch: 0 };
            let mut tex = None;
            self.device.CreateTexture2D(&desc, Some(&init), Some(&mut tex)).expect("texture");
            let tex = tex.unwrap();
            let mut srv = None;
            self.device.CreateShaderResourceView(&tex, None, Some(&mut srv)).expect("srv");
            self.textures.push(Texture { tex, srv: srv.unwrap() });
            self.textures.len() as u32 - 1
        }
    }

    fn update_texture(&mut self, id: u32, x: u32, y: u32, w: u32, h: u32, data: &[u32]) {
        let Some(t) = self.textures.get(id as usize) else { return };
        unsafe {
            let bx = D3D11_BOX { left: x, top: y, front: 0, right: x + w, bottom: y + h, back: 1 };
            self.ctx.UpdateSubresource(&t.tex, 0, Some(&bx), data.as_ptr() as _, w * 4, 0);
        }
    }

    fn draw_textured(&mut self, id: u32, x0: f32, y0: f32, x1: f32, y1: f32, c: Col, linear: bool) {
        self.flush();
        let Some(t) = self.textures.get(id as usize) else { return };
        unsafe {
            self.ctx.PSSetShader(&self.ps_tex, None);
            self.ctx.PSSetShaderResources(0, Some(&[Some(t.srv.clone())]));
            let s = if linear { self.samp_lin.clone() } else { self.samp_pt.clone() };
            self.ctx.PSSetSamplers(0, Some(&[Some(s)]));
        }
        let v = |x, y, u, vv| Vertex { x, y, u, v: vv, c };
        self.verts.extend_from_slice(&[v(x0, y0, 0.0, 0.0), v(x1, y0, 1.0, 0.0), v(x1, y1, 1.0, 1.0), v(x0, y0, 0.0, 0.0), v(x1, y1, 1.0, 1.0), v(x0, y1, 0.0, 1.0)]);
        self.upload_draw();
    }

    fn text_begin(&mut self) {
        self.flush();
        unsafe {
            self.ctx.Flush();
        }
        if let Some(rt) = &self.d2d {
            unsafe {
                rt.BeginDraw();
                let u = self.ui;
                rt.SetTransform(&windows_numerics::Matrix3x2 { M11: u, M12: 0.0, M21: 0.0, M22: u, M31: 0.0, M32: 0.0 });
            }
            self.text_open = true;
        }
    }

    fn text(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, f: Font, c: Col) {
        if s.is_empty() || w <= 0.0 || h <= 0.0 {
            return;
        }
        // 同じ文・書体・枠の配置は覚えておく（毎フレーム組み直すと重い）
        let key = (s.to_string(), f as u8, w.to_bits(), h.to_bits());
        let layout = match self.layout_cache.get(&key) {
            Some(l) => l.clone(),
            None => {
                let wb: Vec<u16> = s.encode_utf16().collect();
                let Ok(l) = (unsafe { self.dwrite.CreateTextLayout(&wb, &self.fonts[f as usize], w, h) }) else { return };
                if self.layout_cache.len() > 2048 {
                    self.layout_cache.clear();
                }
                self.layout_cache.insert(key, l.clone());
                l
            }
        };
        let (Some(rt), Some(b)) = (&self.d2d, &self.brush) else { return };
        unsafe {
            b.SetColor(&D2D1_COLOR_F { r: c[0], g: c[1], b: c[2], a: c[3] });
            rt.DrawTextLayout(windows_numerics::Vector2 { X: x, Y: y }, &layout, b, D2D1_DRAW_TEXT_OPTIONS_NONE);
        }
    }

    fn text_end(&mut self) {
        self.close_text();
    }

    fn measure(&mut self, s: &str, f: Font) -> f32 {
        let key = (s.to_string(), f as u8, 0);
        if let Some(&v) = self.measure_cache.get(&key) {
            return v;
        }
        let wb: Vec<u16> = s.encode_utf16().collect();
        let mut out = s.chars().count() as f32 * 14.0;
        unsafe {
            if let Ok(layout) = self.dwrite.CreateTextLayout(&wb, &self.fonts[f as usize], 4000.0, 200.0) {
                let mut m = DWRITE_TEXT_METRICS::default();
                if layout.GetMetrics(&mut m).is_ok() {
                    out = m.widthIncludingTrailingWhitespace;
                }
            }
        }
        self.measure_cache.insert(key, out);
        out
    }

    fn measure_h(&mut self, s: &str, f: Font, width: f32) -> f32 {
        let key = (s.to_string(), f as u8, width.to_bits());
        if let Some(&v) = self.measure_cache.get(&key) {
            return v;
        }
        let wb: Vec<u16> = s.encode_utf16().collect();
        let mut out = 20.0 * (1 + s.chars().count() / 24) as f32;
        unsafe {
            if let Ok(layout) = self.dwrite.CreateTextLayout(&wb, &self.fonts[f as usize], width, 4000.0) {
                let mut m = DWRITE_TEXT_METRICS::default();
                if layout.GetMetrics(&mut m).is_ok() {
                    out = m.height;
                }
            }
        }
        if self.measure_cache.len() > 4096 {
            self.measure_cache.clear();
        }
        self.measure_cache.insert(key, out);
        out
    }

    fn present(&mut self) {
        self.flush();
        self.close_text();
        unsafe {
            let _ = self.swapchain.Present(1, DXGI_PRESENT(0));
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 || (w == self.pw && h == self.ph) {
            return;
        }
        self.close_text();
        self.rtv = None;
        self.d2d = None;
        self.brush = None;
        unsafe {
            self.ctx.OMSetRenderTargets(None, None);
            self.ctx.Flush();
            if self.swapchain.ResizeBuffers(0, w, h, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0)).is_ok() {
                self.pw = w;
                self.ph = h;
            }
        }
        let _ = self.create_targets();
        self.update_scale();
    }

    fn save_slot(&mut self, slot: u8, data: &[u8], meta: &str) {
        let dir = save_dir();
        let _ = std::fs::create_dir_all(&dir);
        // 途中で落ちても前の記録を壊さないよう、書き終えてから置きかえる
        let path = dir.join(format!("slot{slot}.yysav"));
        let tmp = dir.join(format!("slot{slot}.tmp"));
        if std::fs::write(&tmp, data).is_ok() && std::fs::rename(&tmp, &path).is_ok() {
            let t = unsafe { GetLocalTime() };
            let info = format!("{meta}\t{}/{:02}/{:02} {:02}:{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute);
            let _ = std::fs::write(dir.join(format!("slot{slot}.txt")), &info);
            self.slot_cache.insert(slot, Some(info));
        }
    }

    fn load_slot(&mut self, slot: u8) -> Option<Vec<u8>> {
        std::fs::read(save_dir().join(format!("slot{slot}.yysav"))).ok()
    }

    fn slot_info(&mut self, slot: u8) -> Option<String> {
        self.slot_cache.entry(slot).or_insert_with(|| std::fs::read_to_string(save_dir().join(format!("slot{slot}.txt"))).ok()).clone()
    }

    fn adjust_scale(&mut self, d: i32) {
        self.user_scale = (self.user_scale * 1.12f32.powi(d)).clamp(0.6, 2.2);
        self.update_scale();
    }

    /// ボーダーレス全画面とウィンドウ表示を切り替える
    fn toggle_fullscreen(&mut self) {
        unsafe {
            let hwnd = self.hwnd;
            if let Some((style, rc)) = self.saved_window.take() {
                SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                let _ = SetWindowPos(hwnd, None, rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top, SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOOWNERZORDER);
            } else {
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let mut rc = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rc);
                let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
                if !GetMonitorInfoW(mon, &mut mi).as_bool() {
                    return;
                }
                self.saved_window = Some((style, rc));
                SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !(WS_OVERLAPPEDWINDOW.0 as isize)) | WS_POPUP.0 as isize | WS_VISIBLE.0 as isize);
                let m = mi.rcMonitor;
                let _ = SetWindowPos(hwnd, Some(HWND_TOP), m.left, m.top, m.right - m.left, m.bottom - m.top, SWP_FRAMECHANGED | SWP_NOOWNERZORDER);
            }
        }
    }
}
