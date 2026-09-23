cmake_minimum_required(VERSION 3.22)

project(sunreactor-plasma-panel VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

find_package(ECM 6.0 REQUIRED NO_MODULE)
list(APPEND CMAKE_MODULE_PATH ${ECM_MODULE_PATH})

include(KDEInstallDirs6)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)

find_package(Qt6 REQUIRED COMPONENTS Core Qml Quick Network Test)
find_package(Plasma CONFIG REQUIRED)

qt_policy(SET QTP0001 NEW)

# Plasma 6 package
plasma_install_package(package org.sunreactor.plasma)

# QML C++ Backend Plugin
add_library(sunreactorplugin SHARED)

target_sources(sunreactorplugin PRIVATE
    plugin/sunreactorclient.cpp
    plugin/sunreactorclient.h
    plugin/sunreactortypes.h
)

target_include_directories(sunreactorplugin PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/plugin
)

target_link_libraries(sunreactorplugin
    PRIVATE
        Qt6::Core
        Qt6::Qml
        Qt6::Quick
        Qt6::Network
)

target_compile_definitions(sunreactorplugin PRIVATE QT_NO_CAST_FROM_ASCII)

qt_add_qml_module(sunreactorplugin
    URI org.sunreactor.plasma
    VERSION 1.0
    OUTPUT_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/qml/org/sunreactor/plasma
)

if (TARGET sunreactorpluginplugin)
    set_target_properties(sunreactorpluginplugin PROPERTIES
        PREFIX "lib"
        INSTALL_RPATH "$ORIGIN;${CMAKE_INSTALL_PREFIX}/lib;${CMAKE_INSTALL_PREFIX}/lib/qt6/qml/org/sunreactor/plasma"
    )
endif()

install(
    TARGETS sunreactorplugin
    DESTINATION ${KDE_INSTALL_LIBDIR}
)

install(
    TARGETS sunreactorplugin sunreactorpluginplugin
    DESTINATION ${KDE_INSTALL_QMLDIR}/org/sunreactor/plasma
)

install(
    FILES
        ${CMAKE_CURRENT_BINARY_DIR}/qml/org/sunreactor/plasma/qmldir
        ${CMAKE_CURRENT_BINARY_DIR}/qml/org/sunreactor/plasma/sunreactorplugin.qmltypes
    DESTINATION ${KDE_INSTALL_QMLDIR}/org/sunreactor/plasma
)

install(
    FILES
        package/contents/icons/org.sunreactor.plasma.svg
        package/contents/icons/org.sunreactor.plasma-symbolic.svg
    DESTINATION ${KDE_INSTALL_ICONDIR}/hicolor/scalable/apps
)

# Automated Test Target
enable_testing()

add_executable(test_sunreactorclient
    tests/test_sunreactorclient.cpp
    plugin/sunreactorclient.cpp
)

target_include_directories(test_sunreactorclient PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/plugin
)

target_link_libraries(test_sunreactorclient PRIVATE
    Qt6::Core
    Qt6::Network
    Qt6::Test
)

add_test(NAME test_sunreactorclient COMMAND test_sunreactorclient)
