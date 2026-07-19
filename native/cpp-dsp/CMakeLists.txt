cmake_minimum_required(VERSION 3.16)
project(aeryon-cpp-dsp VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

option(AERYON_DSP_BUILD_TESTS "Build native DSP unit tests" ON)

add_library(aeryon_dsp STATIC
    src/dsp.cpp
)

set_target_properties(aeryon_dsp PROPERTIES
    POSITION_INDEPENDENT_CODE ON
    OUTPUT_NAME aeryon_dsp
)

target_include_directories(aeryon_dsp
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
)

if(MSVC)
    target_compile_options(aeryon_dsp PRIVATE /W4 /permissive-)
else()
    target_compile_options(aeryon_dsp PRIVATE -Wall -Wextra -Wpedantic)
endif()

include(GNUInstallDirs)
install(TARGETS aeryon_dsp
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)
install(FILES include/aeryon/dsp.h include/aeryon/dsp.hpp
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/aeryon
)

if(AERYON_DSP_BUILD_TESTS)
    add_executable(aeryon_dsp_tests
        tests/test_dsp.cpp
    )
    target_link_libraries(aeryon_dsp_tests PRIVATE aeryon_dsp)
    if(MSVC)
        target_compile_options(aeryon_dsp_tests PRIVATE /W4 /permissive-)
    else()
        target_compile_options(aeryon_dsp_tests PRIVATE -Wall -Wextra -Wpedantic)
    endif()

    enable_testing()
    add_test(NAME dsp_kernels COMMAND aeryon_dsp_tests)
endif()
