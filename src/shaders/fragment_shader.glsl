#version 330 core

in vec3 color;
in vec3 norm;
out vec4 out_color;

uniform vec3 light_dir;

void main() {
    float ambient = 0.05;
    float dot_light = dot(normalize(light_dir),norm);
    float ratio = (dot_light + 1.0) / 2.0;
    out_color = vec4(color * ratio + ambient,1.0);
}
