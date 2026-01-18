#version 330 core

in vec4 FragPos;

uniform vec3 lightPos;
uniform float farPlane;

void main() {
    // Store distance to light as depth
    float lightDistance = length(FragPos.xyz - lightPos);
    lightDistance = lightDistance / farPlane; // Normalize to [0,1]
    gl_FragDepth = lightDistance;
}
