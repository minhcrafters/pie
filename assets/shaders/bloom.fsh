#version 330 core
out vec4 FragColor;
in vec2 TexCoords;

uniform sampler2D scene;
uniform float threshold; // luminance threshold for bright-pass

void main() {
    vec3 hdrColor = texture(scene, TexCoords).rgb;

    // Relative luminance (Rec. 709)
    float luminance = dot(hdrColor, vec3(0.2126, 0.7152, 0.0722));

    if (luminance > threshold) {
        FragColor = vec4(hdrColor, 1.0);
    } else {
        FragColor = vec4(0.0, 0.0, 0.0, 1.0);
    }
}
