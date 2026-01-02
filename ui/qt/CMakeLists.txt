cmake_minimum_required(VERSION 3.16)
project(GMTI_Visualizer LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets Network)

add_executable(gmti_visualizer
    src/main.cpp
    src/VisualizationWindow.cpp
    src/InputConfigurator.cpp
    src/StatusGraph.cpp
    src/DataProvider.cpp
)

target_link_libraries(gmti_visualizer PRIVATE Qt6::Widgets Qt6::Network)
