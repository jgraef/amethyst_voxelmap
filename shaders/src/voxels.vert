#version 450

layout(std140, set = 0, binding = 0) uniform VoxelMapArgs {
    uniform mat4 proj;
    uniform mat4 view;
    uniform mat4 map_coordinate_transform;
    uniform mat4 map_transform;
    uniform vec3 voxel_dimensions;
};

// Quad transform.
layout(location = 0) in vec2 tex_top_left;
layout(location = 1) in vec2 tex_bottom_right;
layout(location = 2) in vec4 color;
layout(location = 3) in ivec3 voxel_coordinate;
layout(location = 4) in uint face;

layout(location = 0) out VertexData {
    vec2 tex_uv;
    vec4 color;
} vertex_data;

const vec2 texture_vertices[4] = vec2[](
    vec2(0.5, -0.5), // Right bottom
    vec2(-0.5, -0.5), // Left bottom
    vec2(0.5, 0.5), // Right top
    vec2(-0.5, 0.5) // Left top
);

const vec3 face_vertices[6][4] = vec3[][](
  vec3[](
    vec3(0.5, -0.5, 0.5),
    vec3(-0.5, -0.5, 0.5),
    vec3(0.5, 0.5, 0.5),
    vec3(-0.5, 0.5, 0.5)
  ),
  vec3[](
    vec3(0.5, 0.5, -0.5),
    vec3(-0.5, 0.5, -0.5),
    vec3(0.5, -0.5, -0.5),
    vec3(-0.5, -0.5, -0.5)
  ),
  vec3[](
    vec3(0.5, 0.5, 0.5),
    vec3(-0.5, 0.5, 0.5),
    vec3(0.5, 0.5, -0.5),
    vec3(-0.5, 0.5, -0.5)
  ),
  vec3[](
    vec3(0.5, -0.5, -0.5),
    vec3(-0.5, -0.5, -0.5),
    vec3(0.5, -0.5, 0.5),
    vec3(-0.5, -0.5, 0.5)
  ),
  vec3[](
    vec3(-0.5, -0.5, 0.5),
    vec3(-0.5, -0.5, -0.5),
    vec3(-0.5, 0.5, 0.5),
    vec3(-0.5, 0.5, -0.5)
  ),
  vec3[](
    vec3(0.5, -0.5, -0.5),
    vec3(0.5, -0.5, 0.5),
    vec3(0.5, 0.5, -0.5),
    vec3(0.5, 0.5, 0.5)
  )
);


// Maps the coordinates on the face `uv` to the coordinates in the actual texture.
vec2 texture_coords(vec2 uv, vec2 top_left, vec2 bottom_right) {
    return vec2(
      mix(top_left.x, bottom_right.x, uv.x + 0.5),
      mix(top_left.y, bottom_right.y, uv.y + 0.5)
    );
}

void main() {
    // Voxel coordinate
    vec4 coord = vec4(
      float(voxel_coordinate.x),
      float(voxel_coordinate.y),
      float(voxel_coordinate.z),
      1.0
    );

    // Transform to woorld coordinates.
    // `map_coordinate_transform` centers the voxel map around the entities' transform.
    // `map_transform` is the entities' transform and maps the coordinates to world space.
    vec4 world_coordinate = map_coordinate_transform * coord * transpose(map_transform);

    // Determines the texture coordinates for this vertice in the target texture.
    vertex_data.tex_uv = texture_coords(texture_vertices[gl_VertexIndex], tex_top_left, tex_bottom_right);

    // Set tint
    vertex_data.color = color;

    // Orientation and scaling of voxel map.
    vec3 dir_x = (map_transform[0] * voxel_dimensions.x).xyz;
    vec3 dir_y = (map_transform[1] * voxel_dimensions.y).xyz;
    vec3 dir_z = (map_transform[2] * voxel_dimensions.z).xyz;

    // Face vertex
    vec3 face_vertex = face_vertices[face][gl_VertexIndex];

    // Offset vertex to world coordinates and orientation/scaling of map transform.
    vec4 vertex = vec4(
      world_coordinate.xyz + face_vertex.x * dir_x + face_vertex.y * dir_y + face_vertex.z * dir_z,
      1.0
    );

    gl_Position = proj * view * vertex;
}
