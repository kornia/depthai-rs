// Weak fallback definitions for symbols a DEPTHAI_OPENCV_SUPPORT=OFF build of
// depthai-core v3.7.1 leaves UNDEFINED inside libdepthai-core.so itself
// (ImageFilters / Rectification are only compiled with OpenCV, but other TUs still
// reference them, so the shared library cannot be linked into an executable).
//
// Weak: when depthai-core was built WITH OpenCV the real (strong) definitions win
// and these vanish. Calling any of them throws — this ABI never exposes these
// nodes. Approach after groupe-carvi/depthai-rs's image_filters_stub.cpp (MIT).
#include <stdexcept>
#include <string>

#include "depthai/pipeline/node/ImageFilters.hpp"
#include "depthai/pipeline/node/Rectification.hpp"

#if defined(__ELF__) && (defined(__GNUC__) || defined(__clang__))
#define DAI_WEAK __attribute__((weak))
#else
#define DAI_WEAK
#endif

namespace {
[[noreturn]] void not_available(const char* what) {
    throw std::runtime_error(std::string(what) + " is unavailable: depthai-core was built without OpenCV support");
}
}  // namespace

namespace dai::node {

DAI_WEAK std::shared_ptr<ImageFilters> ImageFilters::build(Node::Output&, ImageFiltersPresetMode) {
    not_available("ImageFilters");
}
DAI_WEAK std::shared_ptr<ImageFilters> ImageFilters::build(ImageFiltersPresetMode) {
    not_available("ImageFilters");
}
// First non-inline virtual: defining it here is what emits the ImageFilters vtable.
DAI_WEAK void ImageFilters::run() {
    not_available("ImageFilters");
}
DAI_WEAK void ImageFilters::setRunOnHost(bool runOnHost) {
    runOnHostVar = runOnHost;
}
DAI_WEAK bool ImageFilters::runOnHost() const {
    return runOnHostVar;
}

DAI_WEAK CalibrationHandler Rectification::getCalibrationData() const {
    not_available("Rectification");
}

}  // namespace dai::node
