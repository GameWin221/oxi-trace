void hash(inout uint seed)
{
    seed ^= 2747636419u;
    seed *= 2654435769u;
    seed ^= seed >> 16u;
    seed *= 2654435769u;
    seed ^= seed >> 16u;
    seed *= 2654435769u;
}
float RandomFloat(inout uint seed)
{
    hash(seed);
    return float(seed)/4294967295.0;
}
float RandomFloatRange(inout uint seed, float min, float max)
{
    return min + (max-min)*RandomFloat(seed);
}
float RandomFloatNormalDist(inout uint seed)
{
    return sqrt(-2.0 * log(RandomFloat(seed))) * cos(2.0 * 3.1415926 * RandomFloat(seed));
}

vec3 RandomVec3(inout uint seed)
{
    return vec3(RandomFloat(seed), RandomFloat(seed), RandomFloat(seed));
}
vec3 RandomVec3Range(inout uint seed, float min, float max)
{
    return vec3(
        RandomFloatRange(seed, min, max),
        RandomFloatRange(seed, min, max),
        RandomFloatRange(seed, min, max)
    );
}
vec3 RandomVec3Direction(inout uint seed)
{
    return normalize(vec3(
        RandomFloatNormalDist(seed),
        RandomFloatNormalDist(seed),
        RandomFloatNormalDist(seed)
    ));
}
vec3 RandomVec3Hemisphere(inout uint seed, vec3 normal)
{
    vec3 direction = RandomVec3Direction(seed);
    return dot(direction, normal) > 0.0 ? direction : -direction;
}