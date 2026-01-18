#version 330 core
out vec4 FragColor;
in vec2 TexCoords;

uniform sampler2D scene;
uniform sampler2D bloomBlur; // blurred bright-pass texture
uniform int toneMappingMode; // 0 = Reinhard, 1 = Filmic
uniform float exposure;
uniform float bloomIntensity; // multiplier for bloom contribution (0.0 = disabled)

void main() {
    const float gamma = 2.2;
    vec3 hdrColor = texture(scene, TexCoords).rgb;

    vec3 bloomColor = texture(bloomBlur, TexCoords).rgb;
    vec3 combined = hdrColor + bloomColor * bloomIntensity;

    vec3 result = vec3(0.0);
    if (toneMappingMode == 0) {
        result = combined / (combined + vec3(1.0));
    } else {
        vec3 x = combined * exposure;
        float a = 2.51;
        float b = 0.03;
        float c = 2.43;
        float d = 0.59;
        float e = 0.14;
        result = clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
    }

    result = pow(result, vec3(1.0 / gamma));
    FragColor = vec4(result, 1.0);
}
