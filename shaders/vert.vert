#version 450

layout (location = 0) in vec2 vPosition;
layout (location = 1) in vec3 vColor;
layout (location = 2) in vec2 vTexCoord;

layout(location = 0) out vec3 fColor;
layout(location = 1) out vec2 fTexCoord;

layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
};

void main() 
{
    gl_Position = proj * view * model * vec4(vPosition, 0.0, 1.0);
    fColor = vColor;
    fTexCoord = vTexCoord;
}