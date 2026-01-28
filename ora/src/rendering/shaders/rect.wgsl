// Styled rectangle shader with rounded corners, borders, shadows, and gradients
// Uses instanced rendering with distance fields for anti-aliased edges

struct RectInstance {
    // Rectangle geometry
    position: vec2<f32>,         // offset 0   — Top-left position in pixels
    size: vec2<f32>,             // offset 8   — Width, height in pixels

    // Visual properties
    color: vec4<f32>,            // offset 16  — Background RGBA
    border_color: vec4<f32>,     // offset 32  — Border RGBA
    border_widths: vec4<f32>,    // offset 48  — top, right, bottom, left
    corners: vec4<f32>,          // offset 64  — TL, TR, BR, BL border-radius

    // Shadow
    shadow_offset: vec2<f32>,    // offset 80
    shadow_blur: f32,            // offset 88
    shadow_spread: f32,          // offset 92
    shadow_color: vec4<f32>,     // offset 96

    // Gradient (if enabled)
    gradient_end_color: vec4<f32>, // offset 112 — If different from color, enables gradient
    gradient_angle: f32,         // offset 128

    // Explicit padding to align window_size to vec2 boundary (8 bytes)
    _pad1: f32,                  // offset 132

    // Viewport for NDC conversion
    window_size: vec2<f32>,      // offset 136

    // Padding to match struct stride (160 bytes, multiple of max alignment 16)
    _pad2: vec4<f32>,            // offset 144
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,       // Position relative to rect center
    @location(1) rect_pos: vec2<f32>,        // Rect top-left
    @location(2) rect_size: vec2<f32>,       // Rect size
    @location(3) color: vec4<f32>,
    @location(4) border_color: vec4<f32>,
    @location(5) border_widths: vec4<f32>,
    @location(6) corners: vec4<f32>,
    @location(7) shadow_offset: vec2<f32>,
    @location(8) shadow_blur: f32,
    @location(9) shadow_spread: f32,
    @location(10) shadow_color: vec4<f32>,
    @location(11) gradient_end_color: vec4<f32>,
    @location(12) gradient_angle: f32,
    @location(13) window_size: vec2<f32>,
}

@group(0) @binding(0)
var<storage, read> instances: array<RectInstance>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let instance = instances[instance_index];

    // Generate quad vertices (two triangles)
    // vertex_index: 0, 1, 2, 3, 4, 5 -> (0,0), (1,0), (0,1), (0,1), (1,0), (1,1)
    var corners_2d = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners_2d[vertex_index];

    // Expand quad to accommodate shadows
    let shadow_expansion = abs(instance.shadow_offset.x) + instance.shadow_blur + instance.shadow_spread;
    let shadow_expansion_y = abs(instance.shadow_offset.y) + instance.shadow_blur + instance.shadow_spread;
    let expanded_size = instance.size + vec2<f32>(shadow_expansion * 2.0, shadow_expansion_y * 2.0);
    let expanded_pos = instance.position - vec2<f32>(shadow_expansion, shadow_expansion_y);

    // Compute pixel position
    let pixel_pos = expanded_pos + corner * expanded_size;

    // Convert to NDC (normalized device coordinates)
    // wgpu uses: x: -1 (left) to +1 (right), y: -1 (top) to +1 (bottom)
    let x_ndc = (pixel_pos.x / instance.window_size.x) * 2.0 - 1.0;
    let y_ndc = 1.0 - (pixel_pos.y / instance.window_size.y) * 2.0; // Y-flip for wgpu

    // Local position relative to rect center (for distance field calculations)
    let rect_center = instance.position + instance.size * 0.5;
    let local = pixel_pos - rect_center;

    var output: VertexOutput;
    output.clip_position = vec4<f32>(x_ndc, y_ndc, 0.0, 1.0);
    output.local_pos = local;
    output.rect_pos = instance.position;
    output.rect_size = instance.size;
    output.color = instance.color;
    output.border_color = instance.border_color;
    output.border_widths = instance.border_widths;
    output.corners = instance.corners;
    output.shadow_offset = instance.shadow_offset;
    output.shadow_blur = instance.shadow_blur;
    output.shadow_spread = instance.shadow_spread;
    output.shadow_color = instance.shadow_color;
    output.gradient_end_color = instance.gradient_end_color;
    output.gradient_angle = instance.gradient_angle;
    output.window_size = instance.window_size;

    return output;
}

// Signed distance to rounded rectangle edge
fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - radius;
}

// Select corner radius based on pixel position
fn select_corner_radius(p: vec2<f32>, corners: vec4<f32>) -> f32 {
    // corners: (top_left, top_right, bottom_right, bottom_left)
    if p.x < 0.0 && p.y < 0.0 {
        return corners.x; // Top-left
    } else if p.x >= 0.0 && p.y < 0.0 {
        return corners.y; // Top-right
    } else if p.x >= 0.0 && p.y >= 0.0 {
        return corners.z; // Bottom-right
    } else {
        return corners.w; // Bottom-left
    }
}

// Error function approximation for Gaussian shadow
fn erf_approx(x: f32) -> f32 {
    let s = sign(x);
    let a = abs(x);
    var val = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    val = val * val;
    return s - s / (val * val);
}

// Closed-form box shadow using Gaussian integral
fn box_shadow_alpha(p: vec2<f32>, half_size: vec2<f32>, blur: f32, spread: f32) -> f32 {
    if blur <= 0.0 {
        return 0.0;
    }

    let expanded = half_size + vec2<f32>(spread);
    let d = abs(p) - expanded;

    // Use error function for closed-form Gaussian integral
    let sigma = blur * 0.5;
    let integral_x = 0.5 * (erf_approx((d.x + blur) / (sigma * 1.414213)) - erf_approx((d.x - blur) / (sigma * 1.414213)));
    let integral_y = 0.5 * (erf_approx((d.y + blur) / (sigma * 1.414213)) - erf_approx((d.y - blur) / (sigma * 1.414213)));

    return clamp(integral_x * integral_y, 0.0, 1.0);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let half_size = input.rect_size * 0.5;
    let corner_radius = select_corner_radius(input.local_pos, input.corners);

    // Distance to rounded rectangle edge
    let dist = rounded_rect_sdf(input.local_pos, half_size, corner_radius);

    // Shadow rendering (behind the rectangle)
    var shadow_alpha = 0.0;
    if input.shadow_blur > 0.0 {
        let shadow_pos = input.local_pos - input.shadow_offset;
        shadow_alpha = box_shadow_alpha(shadow_pos, half_size, input.shadow_blur, input.shadow_spread);

        // Shadow only visible outside the rectangle
        if dist < 0.0 {
            shadow_alpha = 0.0;
        }
    }

    // Background color (with optional gradient)
    var bg_color = input.color;
    if input.gradient_end_color.a > 0.0 || length(input.gradient_end_color.rgb - input.color.rgb) > 0.01 {
        // Gradient enabled
        let uv = (input.local_pos + half_size) / input.rect_size; // Normalize to 0..1
        let angle = input.gradient_angle;
        let rotated = uv.x * cos(angle) - uv.y * sin(angle);
        let t = clamp(rotated, 0.0, 1.0);
        bg_color = mix(input.color, input.gradient_end_color, t);
    }

    // Border detection (distance-based)
    let max_border = max(max(input.border_widths.x, input.border_widths.y),
                         max(input.border_widths.z, input.border_widths.w));
    var is_border = false;
    var border_width = 0.0;

    if dist < 0.0 && dist > -max_border {
        // Inside rect, possibly in border region
        // Select border width based on which edge is closest
        let edge_dist = abs(input.local_pos) - half_size;
        if abs(edge_dist.y - max(edge_dist.x, edge_dist.y)) < 0.1 {
            // Top or bottom edge
            if input.local_pos.y < 0.0 {
                border_width = input.border_widths.x; // Top
            } else {
                border_width = input.border_widths.z; // Bottom
            }
        } else {
            // Left or right edge
            if input.local_pos.x < 0.0 {
                border_width = input.border_widths.w; // Left
            } else {
                border_width = input.border_widths.y; // Right
            }
        }

        if dist > -border_width {
            is_border = true;
        }
    }

    // Anti-aliasing with smoothstep
    let edge_distance = abs(dist);
    let aa_width = 1.0; // 1 pixel transition
    var alpha = 1.0 - smoothstep(-aa_width * 0.5, aa_width * 0.5, dist);

    // Compose final color
    var final_color: vec4<f32>;

    if dist >= 0.0 {
        // Outside rectangle - only shadow
        final_color = vec4<f32>(input.shadow_color.rgb, input.shadow_color.a * shadow_alpha);
    } else if is_border {
        // Border region
        final_color = vec4<f32>(input.border_color.rgb, input.border_color.a * alpha);
    } else {
        // Inside background
        final_color = vec4<f32>(bg_color.rgb, bg_color.a * alpha);
    }

    return final_color;
}
