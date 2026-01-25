// Styled Rectangle Shader with SDF-based rendering
//
// This shader renders rectangles with:
// - Per-corner border radii
// - Anti-aliased edges using signed distance functions
// - Soft drop shadows with blur
// - Borders with adjustable width
//
// Coordinate system:
// - Input coordinates are in logical pixels
// - Converted to NDC (-1 to 1) in the vertex shader

// Instance data from StyledRectInstance
struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) fill_color: vec4<f32>,
    @location(3) border_color: vec4<f32>,
    @location(4) border_width: f32,
    @location(5) corner_radii: vec4<f32>,  // [tl, tr, br, bl]
    @location(6) shadow_color: vec4<f32>,
    @location(7) shadow_offset: vec2<f32>,
    @location(8) shadow_blur: f32,
    @location(9) shadow_spread: f32,
    @location(10) screen_size: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,      // Position relative to rect center
    @location(1) rect_size: vec2<f32>,      // Half-size of the rect
    @location(2) fill_color: vec4<f32>,
    @location(3) border_color: vec4<f32>,
    @location(4) border_width: f32,
    @location(5) corner_radii: vec4<f32>,
    @location(6) shadow_color: vec4<f32>,
    @location(7) shadow_offset: vec2<f32>,
    @location(8) shadow_blur: f32,
    @location(9) shadow_spread: f32,
};

// Quad vertices (we generate a quad per instance)
const QUAD_VERTICES: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0),  // Triangle 1
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(0.0, 0.0),  // Triangle 2
    vec2<f32>(1.0, 1.0),
    vec2<f32>(0.0, 1.0),
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Get base quad vertex (0-1 range)
    let quad_pos = QUAD_VERTICES[vertex_index];

    // Calculate shadow expansion (shadow needs extra space)
    let shadow_expansion = instance.shadow_blur + abs(instance.shadow_spread) +
                          max(abs(instance.shadow_offset.x), abs(instance.shadow_offset.y));

    // Expand the quad to accommodate shadow
    let expanded_size = instance.size + vec2<f32>(shadow_expansion * 2.0);
    let expanded_pos = instance.pos - vec2<f32>(shadow_expansion);

    // Calculate pixel position
    let pixel_pos = expanded_pos + quad_pos * expanded_size;

    // Convert to NDC
    let ndc = (pixel_pos / instance.screen_size) * 2.0 - 1.0;
    out.position = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);  // Flip Y for screen coords

    // Calculate local position relative to rect center
    let rect_center = instance.pos + instance.size * 0.5;
    out.local_pos = pixel_pos - rect_center;
    out.rect_size = instance.size * 0.5;

    // Pass through instance data
    out.fill_color = instance.fill_color;
    out.border_color = instance.border_color;
    out.border_width = instance.border_width;
    out.corner_radii = instance.corner_radii;
    out.shadow_color = instance.shadow_color;
    out.shadow_offset = instance.shadow_offset;
    out.shadow_blur = instance.shadow_blur;
    out.shadow_spread = instance.shadow_spread;

    return out;
}

// Signed distance function for a rounded rectangle
// p: point relative to rect center
// b: half-size of the rect
// r: corner radii [tl, tr, br, bl]
fn sd_rounded_rect(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    // Select the appropriate radius based on quadrant
    var radius: f32;
    if (p.x < 0.0) {
        if (p.y < 0.0) {
            radius = r.x;  // top-left
        } else {
            radius = r.w;  // bottom-left
        }
    } else {
        if (p.y < 0.0) {
            radius = r.y;  // top-right
        } else {
            radius = r.z;  // bottom-right
        }
    }

    // Clamp radius to half the smaller dimension
    radius = min(radius, min(b.x, b.y));

    // Compute SDF for rounded rectangle
    let q = abs(p) - b + radius;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var final_color = vec4<f32>(0.0);

    // Calculate anti-aliasing width based on derivatives
    let aa_width = fwidth(in.local_pos.x) * 1.5;

    // === SHADOW ===
    if (in.shadow_color.a > 0.0 && in.shadow_blur > 0.0) {
        // Shadow is offset and potentially expanded
        let shadow_pos = in.local_pos - in.shadow_offset;
        let shadow_size = in.rect_size + in.shadow_spread;

        // Get distance for shadow
        let shadow_dist = sd_rounded_rect(shadow_pos, shadow_size, in.corner_radii);

        // Soft shadow using Gaussian-like falloff
        let shadow_alpha = 1.0 - smoothstep(-in.shadow_blur, in.shadow_blur, shadow_dist);
        let shadow = in.shadow_color * shadow_alpha;

        // Blend shadow (shadow goes behind everything)
        final_color = shadow;
    }

    // === FILL + BORDER ===
    let dist = sd_rounded_rect(in.local_pos, in.rect_size, in.corner_radii);

    // Fill (inside the shape)
    if (in.fill_color.a > 0.0) {
        let fill_alpha = 1.0 - smoothstep(-aa_width, aa_width, dist);
        let fill = vec4<f32>(in.fill_color.rgb, in.fill_color.a * fill_alpha);

        // Blend fill over shadow
        final_color = blend_over(fill, final_color);
    }

    // Border (ring around the shape)
    if (in.border_width > 0.0 && in.border_color.a > 0.0) {
        // Border is the area between outer edge and inner edge
        let inner_dist = dist + in.border_width;

        // Outer edge of border (anti-aliased)
        let outer_alpha = 1.0 - smoothstep(-aa_width, aa_width, dist);
        // Inner edge of border (anti-aliased)
        let inner_alpha = 1.0 - smoothstep(-aa_width, aa_width, inner_dist);

        // Border is where we're inside outer but outside inner
        let border_alpha = outer_alpha - inner_alpha;

        if (border_alpha > 0.0) {
            let border = vec4<f32>(in.border_color.rgb, in.border_color.a * border_alpha);
            final_color = blend_over(border, final_color);
        }
    }

    return final_color;
}

// Alpha compositing: blend foreground over background
fn blend_over(fg: vec4<f32>, bg: vec4<f32>) -> vec4<f32> {
    let alpha = fg.a + bg.a * (1.0 - fg.a);
    if (alpha < 0.001) {
        return vec4<f32>(0.0);
    }
    let rgb = (fg.rgb * fg.a + bg.rgb * bg.a * (1.0 - fg.a)) / alpha;
    return vec4<f32>(rgb, alpha);
}
