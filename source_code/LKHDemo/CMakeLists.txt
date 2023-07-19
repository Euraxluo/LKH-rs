CMAKE_MINIMUM_REQUIRED(VERSION 2.8)
MESSAGE(STATUS "CMAKE_VERSION:${CMAKE_VERSION}")

#set(CMAKE_CXX_FLAGS "-O3 -Og -Wall -g -fPIC -ggdb ")
#set(CFLAGS          "-O3 -Og -Wall -g -fPIC -ggdb ")
#MESSAGE(STATUS "CFLAGS:${CFLAGS}")
#MESSAGE(STATUS "CMAKE_CXX_FLAGS:${CMAKE_CXX_FLAGS}")

SET(TARGET_NAME LKHTool)
PROJECT(${TARGET_NAME})

MESSAGE(STATUS "TARGET_NAME:${TARGET_NAME}")
include_directories(${PROJECT_SOURCE_DIR}/SRC/INCLUDE)

SET(SRC_LISTS 
    ./SRC/GetTime.c
    ./SRC/demo.c
    ./SRC/demo2.c
    ./SRC/LKHmain.cpp
)

MESSAGE(STATUS "Build So")
add_library(${TARGET_NAME} SHARED ${SRC_LISTS})

#mkdir build && cd build
#cmake .. && make
#cd ../SRC && gcc -o ../build/LKH LKHmain.c -I./INCLUDE -L../build  -Wl,-rpath=../build  -l LKHTool
#OSX:cd ../SRC && gcc -o ../build/LKH LKHmain.c -I./INCLUDE -L../build    -l LKHTool
#../build/LKH
