#version 330 core
layout(location = 0) out vec4 FragColor;

in vec2 TexCoords;

uniform sampler2D gPosition;
uniform sampler2D gNormal;
uniform sampler2D gAlbedoSpec;
uniform sampler2D directionalShadowMap;
uniform samplerCube pointShadowMaps[16];

float ShadowCalculationDirectional(vec4 fragPosLightSpace, vec3 normal, vec3 lightDir) {
    vec3 projCoords = fragPosLightSpace.xyz / fragPosLightSpace.w;
    projCoords = projCoords * 0.5 + 0.5;
    float currentDepth = projCoords.z;

    if (projCoords.z > 1.0)
        return 0.0;

    float bias = max(0.005 * (1.0 - dot(normal, lightDir)), 0.0005);

    ivec2 texSize = textureSize(directionalShadowMap, 0);
    vec2 texelSize = 1.0 / vec2(texSize);
    float shadow = 0.0;
    for (int x = -1; x <= 1; ++x) {
        for (int y = -1; y <= 1; ++y) {
            float pcfDepth = texture(directionalShadowMap, projCoords.xy + vec2(x, y) * texelSize).r;
            if (currentDepth - bias > pcfDepth)
                shadow += 1.0;
        }
    }
    shadow /= 9.0;

    return shadow;
}

float ShadowCalculationPoint(vec3 fragPos, vec3 lightPos, float farPlane, int lightIndex, vec3 normal) {
    vec3 fragToLight = fragPos - lightPos;
    float currentDepth = length(fragToLight);

    float closestDepth = texture(pointShadowMaps[lightIndex], normalize(fragToLight)).r;
    closestDepth *= farPlane;

    vec3 lightDir = normalize(lightPos - fragPos);
    float bias = max(0.1 * (1.0 - dot(normal, lightDir)), 0.05);

    float shadow = currentDepth - bias > closestDepth ? 1.0 : 0.0;
    return shadow;
}

struct Light {
    vec3 Position;
    vec3 Color;
    float Radius;
    int Type; // 0 = Point, 1 = Directional
    int HasShadow; // 0 = No shadow, 1 = Has shadow
    int ShadowMapIndex; // Index into shadow map array
};
const int NR_LIGHTS = 32;
uniform Light lights[NR_LIGHTS];
uniform int numLights;
uniform vec3 viewPos;
uniform mat4 lightSpaceMatrix;
uniform mat4 directionalLightSpaceMatrix;
uniform vec3 directionalLightDir;
uniform float farPlane;

void main() {
    vec3 FragPos = texture(gPosition, TexCoords).rgb;
    vec3 Normal = texture(gNormal, TexCoords).rgb;
    vec3 Diffuse = texture(gAlbedoSpec, TexCoords).rgb;
    float Specular = texture(gAlbedoSpec, TexCoords).a;

    vec3 lighting = Diffuse * 0.1; // ambient
    vec3 viewDir = normalize(viewPos - FragPos);

    for (int i = 0; i < numLights; ++i) {
        vec3 lightDir;
        float attenuation = 1.0;

        if (lights[i].Type == 1) {
            lightDir = normalize(-lights[i].Position);
            attenuation = 1.0;
        } else {
            lightDir = normalize(lights[i].Position - FragPos);
            float distance = length(lights[i].Position - FragPos);
            float radius = lights[i].Radius;
            float linear = 4.5 / radius;
            float quadratic = 75.0 / (radius * radius);
            attenuation = 1.0 / (1.0 + linear * distance + quadratic * distance * distance);
            float fadeStart = radius * 0.9;
            if (distance >= radius) {
                attenuation = 0.0;
            } else {
                float fade = 1.0 - smoothstep(fadeStart, radius, distance);
                attenuation *= fade;
            }
        }

        if (attenuation > 0.0) {
            float shadow = 0.0;
            if (lights[i].HasShadow == 1) {
                if (lights[i].Type == 1) {
                    vec4 fragPosLightSpace = directionalLightSpaceMatrix * vec4(FragPos, 1.0);
                    shadow = ShadowCalculationDirectional(fragPosLightSpace, Normal, lightDir);
                } else {
                    shadow = ShadowCalculationPoint(FragPos, lights[i].Position, farPlane, lights[i].ShadowMapIndex, Normal);
                }
            }

            float diff = max(dot(Normal, lightDir), 0.0);
            vec3 diffuse = diff * Diffuse * lights[i].Color;

            vec3 halfwayDir = normalize(lightDir + viewDir);
            float spec = pow(max(dot(Normal, halfwayDir), 0.0), 16.0);
            vec3 specular = lights[i].Color * spec * Specular;

            float radiusScale = (lights[i].Type == 0) ? max(lights[i].Radius, 1.0) : 1.0;
            lighting += (diffuse + specular) * attenuation * radiusScale * (1.0 - shadow);
        }
    }

    FragColor = vec4(lighting, 1.0);

}
