in vec2 v_uv;
in vec4 v_color;
out vec4 frag;

uniform sampler2D tex_1;

void main() {
    vec4 color = texture(tex_1, v_uv);
    frag = color;
}