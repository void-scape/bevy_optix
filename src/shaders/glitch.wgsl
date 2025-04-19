#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_render::globals::Globals

@group(0) @binding(1) var<uniform> globals: Globals;

struct Uniforms {
    shake_power: f32,
    shake_rate: f32,
    shake_speed: f32,
    shake_block_size: f32,
    shake_color_rate: f32,
    intensity: f32,
};

@group(2) @binding(0) var screen_texture: texture_2d<f32>;
@group(2) @binding(1) var screen_sampler: sampler;
@group(2) @binding(2) var<uniform> uniforms: Uniforms;

fn random(seed: f32) -> f32 {
    let dot_product = seed * 3525.46 + seed * -54.3415;
    return fract(543.2543 * sin(dot_product));
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let intensity = uniforms.intensity;

    // Calculate the fixed UV coordinates with shake effect
    var fixed_uv = mesh.uv;
    fixed_uv.x += (
        random(
            (trunc(mesh.uv.y * uniforms.shake_block_size) / uniforms.shake_block_size) +
            globals.time
        ) - 0.5
    ) * uniforms.shake_power * intensity;

    // Sample the main color
    let pixel_color = textureSample(screen_texture, screen_sampler, fixed_uv);

    // Sample colors for chromatic aberration
    let color_r = textureSample(
        screen_texture,
        screen_sampler,
        fixed_uv + vec2<f32>(uniforms.shake_color_rate, 0.0)
    ).r;

    let color_b = textureSample(
        screen_texture,
        screen_sampler,
        fixed_uv + vec2<f32>(-uniforms.shake_color_rate, 0.0)
    ).b;

    // Mix the colors based on intensity
    let final_r = mix(pixel_color.r, color_r, intensity);
    let final_b = mix(pixel_color.b, color_b, intensity);

    return vec4<f32>(final_r, pixel_color.g, final_b, pixel_color.a);
}
