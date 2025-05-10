#version 430

vec2 positions[4] = vec2[](
    vec2(-1.0, -1.0),  
    vec2(-1.0,  1.0),  
    vec2( 1.0,  1.0),  
    vec2( 1.0, -1.0)   
);

vec2 uvs[4] = vec2[](
    vec2(0.0, 0.0),  
    vec2(0.0, 1.0), 
    vec2(1.0, 1.0),  
    vec2(1.0, 0.0)   
);

out vec2 TexCoords; 

void main() {
    gl_Position = vec4(positions[gl_VertexID],0.0, 1.0); 
    TexCoords = uvs[gl_VertexID];
}
