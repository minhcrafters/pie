#version 330 core

layout (location = 0) out vec4 FragColor;

uniform vec3 color;
uniform float intensity;

void main() {
    vec3 hdr = color * intensity;
    FragColor = vec4(hdr, 1.0);
}
