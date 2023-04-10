%VULKAN_SDK%/Bin/glslc.exe %~dp0\vert.vert -o %~dp0\vert.spv
%VULKAN_SDK%/Bin/glslc.exe %~dp0\frag.frag -o %~dp0\frag.spv
%VULKAN_SDK%/Bin/glslc.exe %~dp0\main.comp -o %~dp0\main.spv

pause