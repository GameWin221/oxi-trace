#version 450

layout(location = 0) in vec3 fColor;
layout(location = 1) in vec2 fTexCoord;

layout(location = 0) out vec4 FragColor;

layout (binding = 1) uniform sampler2D mainTexture;

void main() 
{
    FragColor = vec4(texture(mainTexture, fTexCoord).rgb, 1.0);
}