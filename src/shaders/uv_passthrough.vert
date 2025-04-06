#version 430

layout(location = 0) in vec2 position; // Vertex position
layout(location = 1) in vec2 texCoord; // Texture coordinates

out vec2 TexCoords; // Output texture coordinates to the fragment shader

void main() {
    gl_Position = vec4(position,0.0, 1.0); // Transform to clip space
    // Pass the texture coordinates to the fragment shader
    TexCoords = texCoord;
    // Set the position of the vertex
}
