#!/bin/sh

cd shaders

glslc -g -c -O -o compiled/voxels.vert.spv src/voxels.vert
glslc -g -c -O -o compiled/voxels.frag.spv src/voxels.frag

