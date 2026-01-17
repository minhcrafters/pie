#version 330 core
layout(location = 0) out vec3 gPosition;
layout(location = 1) out vec3 gNormal;
layout(location = 2) out vec4 gAlbedoSpec;

in vec3 FragPos;
in vec3 Normal;

// Per-mesh color mapping (rgb = diffuse color, a = specular intensity)
uniform vec4 albedoColor;

void main() {
    // store the fragment position vector in the first gbuffer texture
    gPosition = FragPos;

    // also store the per-fragment normals into the gbuffer
    gNormal = normalize(Normal);

    // Use per-mesh constant color provided by the application.
    vec3 diffuse = albedoColor.rgb;
    float specularIntensity = albedoColor.a;

    // write to G-buffer
    gAlbedoSpec.rgb = diffuse;
    // store specular intensity in gAlbedoSpec's alpha component
    gAlbedoSpec.a = specularIntensity;
}
