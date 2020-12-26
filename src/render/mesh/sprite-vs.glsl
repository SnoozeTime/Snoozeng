in vec2 position;
in vec2 uv;
in vec4 color;

out vec4 v_color;
out vec2 v_uv;

uniform mat4 u_projection;
uniform mat4 u_view;
uniform mat4 u_model;

uniform float u_columns;
uniform float u_rows;
uniform float u_sprite_nb;

void main() {
    vec2 uv = uv;
    float uv_width = 1.0 / u_columns;
    float uv_height = 1.0 / u_rows;

    // Find (x,y) from sprite nb.
    float y = floor(u_sprite_nb / u_columns); // integer division
    float x = u_sprite_nb - y * int(u_columns);

    v_uv.x = (uv.x + x) * uv_width;
    v_uv.y = uv.y * uv_height + 1 - (1 + y) * uv_height;
    v_color = color;
    gl_Position = u_projection * u_view *  u_model  * vec4(position, 0.0, 1.0);
}