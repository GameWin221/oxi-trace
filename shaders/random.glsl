void hash(inout uint seed)
{
    seed ^= 2747636419u;
    seed *= 2654435769u;
    seed ^= seed >> 16u;
    seed *= 2654435769u;
    seed ^= seed >> 16u;
    seed *= 2654435769u;
}
float randomFloat(inout uint seed)
{
    hash(seed);
    return float(seed)/4294967295.0;
}
float randomFloatRange(inout uint seed, float min, float max)
{
    return min + (max-min)*randomFloat(seed);
}
float randomFloatNormalDist(inout uint seed)
{
    return sqrt(-2.0 * log(randomFloat(seed))) * cos(2.0 * 3.1415926 * randomFloat(seed));
}

vec3 randomVec3(inout uint seed)
{
    return vec3(randomFloat(seed), randomFloat(seed), randomFloat(seed));
}
vec3 randomVec3Range(inout uint seed, float min, float max)
{
    return vec3(
        randomFloatRange(seed, min, max),
        randomFloatRange(seed, min, max),
        randomFloatRange(seed, min, max)
    );
}
vec3 randomVec3Direction(inout uint seed)
{
    return normalize(vec3(
        randomFloatNormalDist(seed),
        randomFloatNormalDist(seed),
        randomFloatNormalDist(seed)
    ));
}