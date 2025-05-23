#version 330 core
layout (location = 0) in vec3 position;
layout (location = 1) in vec3 vcolor;
layout (location = 2) in vec3 vnorm;

out vec3 color;
out vec3 norm;

uniform mat4 transform;

void main() {
    gl_Position = transform * vec4(position, 1.0);
    color = vcolor;
    norm = vnorm;
}
