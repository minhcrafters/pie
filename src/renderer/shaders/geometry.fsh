#version 330 core
layout(location = 0) out vec3 gPosition;
layout(location = 1) out vec3 gNormal;
layout(location = 2) out vec4 gAlbedoSpec;

in vec3 FragPos;
in vec3 Normal;
in vec2 TexCoords;

uniform sampler2D albedoMap;

uniform vec4 albedoColor;

void main() {
    gPosition = FragPos;

    gNormal = normalize(Normal);

    vec4 texColor = texture(albedoMap, TexCoords);
    vec3 diffuse = texColor.rgb * albedoColor.rgb;
    float specularIntensity = albedoColor.a;

    gAlbedoSpec.rgb = diffuse;
    gAlbedoSpec.a = specularIntensity;
}
