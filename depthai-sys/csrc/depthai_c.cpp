// depthai_c.cpp — implementation of the pure-C ABI declared in depthai_c.h over
// depthai-core v3. This is the only translation unit that sees C++ from depthai.
//
// Every entry point is wrapped in DAI_GUARD: exceptions become DAI_ERR plus a
// thread-local message. No policy lives here (see the header).

#include "depthai_c.h"

#include <algorithm>
#include <chrono>
#include <climits>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <memory>
#include <mutex>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

// Only the headers this ABI mirrors — NOT the depthai/depthai.hpp umbrella. That
// umbrella pulls in every node, and some (ImageFilters, Rectification) carry
// inline code referencing symbols a DEPTHAI_OPENCV_SUPPORT=OFF build never
// compiles, which surfaces as undefined vtables at link time.
#include "depthai/build/version.hpp"
#include "depthai/device/CalibrationHandler.hpp"
#include "depthai/device/Device.hpp"
#include "depthai/device/DeviceBootloader.hpp"
#include "depthai/pipeline/MessageQueue.hpp"
#include "depthai/pipeline/Node.hpp"
#include "depthai/pipeline/Pipeline.hpp"
#include "depthai/pipeline/datatype/ADatatype.hpp"
#include "depthai/pipeline/datatype/Buffer.hpp"
#include "depthai/pipeline/InputQueue.hpp"
#include "depthai/pipeline/datatype/EncodedFrame.hpp"
#include "depthai/pipeline/datatype/GateControl.hpp"
#include "depthai/pipeline/datatype/IMUData.hpp"
#include "depthai/pipeline/datatype/ImgFrame.hpp"
#include "depthai/pipeline/datatype/MessageGroup.hpp"
#include "depthai/pipeline/datatype/StereoDepthConfig.hpp"
#include "depthai/pipeline/node/Camera.hpp"
#include "depthai/modelzoo/Zoo.hpp"
#include "depthai/nn_archive/NNArchive.hpp"
#include "depthai/pipeline/datatype/ImgDetections.hpp"
#include "depthai/pipeline/datatype/NNData.hpp"
#include "depthai/pipeline/node/DetectionNetwork.hpp"
#include "depthai/pipeline/node/Gate.hpp"
#include "depthai/pipeline/node/NeuralNetwork.hpp"
#include "depthai/pipeline/node/IMU.hpp"
#include "depthai/pipeline/node/StereoDepth.hpp"
#include "depthai/pipeline/node/Sync.hpp"
#include "depthai/pipeline/node/VideoEncoder.hpp"

// ---------------------------------------------------------------------------
// Enum pins: a depthai-core bump that renumbers an enumerator fails HERE.
// ---------------------------------------------------------------------------
#define DAI_PIN(cpp, c) static_assert((int32_t)(cpp) == (c), "depthai_c.h constant out of sync with depthai-core: " #cpp)

DAI_PIN(dai::CameraBoardSocket::AUTO, DAI_CAM_AUTO);
DAI_PIN(dai::CameraBoardSocket::CAM_A, DAI_CAM_A);
DAI_PIN(dai::CameraBoardSocket::CAM_B, DAI_CAM_B);
DAI_PIN(dai::CameraBoardSocket::CAM_C, DAI_CAM_C);
DAI_PIN(dai::CameraBoardSocket::CAM_D, DAI_CAM_D);
DAI_PIN(dai::CameraBoardSocket::CAM_E, DAI_CAM_E);
DAI_PIN(dai::CameraBoardSocket::CAM_F, DAI_CAM_F);
DAI_PIN(dai::CameraBoardSocket::CAM_G, DAI_CAM_G);
DAI_PIN(dai::CameraBoardSocket::CAM_H, DAI_CAM_H);

DAI_PIN(dai::UsbSpeed::UNKNOWN, DAI_USB_UNKNOWN);
DAI_PIN(dai::UsbSpeed::LOW, DAI_USB_LOW);
DAI_PIN(dai::UsbSpeed::FULL, DAI_USB_FULL);
DAI_PIN(dai::UsbSpeed::HIGH, DAI_USB_HIGH);
DAI_PIN(dai::UsbSpeed::SUPER, DAI_USB_SUPER);
DAI_PIN(dai::UsbSpeed::SUPER_PLUS, DAI_USB_SUPER_PLUS);

DAI_PIN(dai::ImgResizeMode::CROP, DAI_RESIZE_CROP);
DAI_PIN(dai::ImgResizeMode::STRETCH, DAI_RESIZE_STRETCH);
DAI_PIN(dai::ImgResizeMode::LETTERBOX, DAI_RESIZE_LETTERBOX);

DAI_PIN(dai::ImgFrame::Type::YUV420p, DAI_IMG_YUV420P);
DAI_PIN(dai::ImgFrame::Type::RGB888p, DAI_IMG_RGB888P);
DAI_PIN(dai::ImgFrame::Type::BGR888p, DAI_IMG_BGR888P);
DAI_PIN(dai::ImgFrame::Type::RGB888i, DAI_IMG_RGB888I);
DAI_PIN(dai::ImgFrame::Type::BGR888i, DAI_IMG_BGR888I);
DAI_PIN(dai::ImgFrame::Type::RAW16, DAI_IMG_RAW16);
DAI_PIN(dai::ImgFrame::Type::RAW8, DAI_IMG_RAW8);
DAI_PIN(dai::ImgFrame::Type::NV12, DAI_IMG_NV12);
DAI_PIN(dai::ImgFrame::Type::BITSTREAM, DAI_IMG_BITSTREAM);
DAI_PIN(dai::ImgFrame::Type::GRAY8, DAI_IMG_GRAY8);
DAI_PIN(dai::ImgFrame::Type::NONE, DAI_IMG_NONE);

DAI_PIN(dai::DatatypeEnum::ADatatype, DAI_DT_ADATATYPE);
DAI_PIN(dai::DatatypeEnum::Buffer, DAI_DT_BUFFER);
DAI_PIN(dai::DatatypeEnum::ImgFrame, DAI_DT_IMG_FRAME);
DAI_PIN(dai::DatatypeEnum::EncodedFrame, DAI_DT_ENCODED_FRAME);
DAI_PIN(dai::DatatypeEnum::GateControl, DAI_DT_GATE_CONTROL);
DAI_PIN(dai::DatatypeEnum::NNData, DAI_DT_NN_DATA);
DAI_PIN(dai::DatatypeEnum::ImgDetections, DAI_DT_IMG_DETECTIONS);

DAI_PIN(dai::TensorInfo::DataType::FP16, DAI_TENSOR_FP16);
DAI_PIN(dai::TensorInfo::DataType::U8F, DAI_TENSOR_U8F);
DAI_PIN(dai::TensorInfo::DataType::INT, DAI_TENSOR_INT);
DAI_PIN(dai::TensorInfo::DataType::FP32, DAI_TENSOR_FP32);
DAI_PIN(dai::TensorInfo::DataType::I8, DAI_TENSOR_I8);
DAI_PIN(dai::TensorInfo::DataType::FP64, DAI_TENSOR_FP64);
DAI_PIN(dai::TensorInfo::DataType::U16F, DAI_TENSOR_U16F);
DAI_PIN(dai::TensorInfo::StorageOrder::NHWC, DAI_ORDER_NHWC);
DAI_PIN(dai::TensorInfo::StorageOrder::NHCW, DAI_ORDER_NHCW);
DAI_PIN(dai::TensorInfo::StorageOrder::NCHW, DAI_ORDER_NCHW);
DAI_PIN(dai::TensorInfo::StorageOrder::HWC, DAI_ORDER_HWC);
DAI_PIN(dai::TensorInfo::StorageOrder::CHW, DAI_ORDER_CHW);
DAI_PIN(dai::TensorInfo::StorageOrder::NC, DAI_ORDER_NC);
DAI_PIN(dai::TensorInfo::StorageOrder::C, DAI_ORDER_C);
DAI_PIN(dai::DatatypeEnum::IMUData, DAI_DT_IMU_DATA);
DAI_PIN(dai::DatatypeEnum::MessageGroup, DAI_DT_MESSAGE_GROUP);

DAI_PIN(dai::CameraModel::Perspective, DAI_CAMERA_MODEL_PERSPECTIVE);
DAI_PIN(dai::CameraModel::Fisheye, DAI_CAMERA_MODEL_FISHEYE);
DAI_PIN(dai::CameraModel::Equirectangular, DAI_CAMERA_MODEL_EQUIRECTANGULAR);
DAI_PIN(dai::CameraModel::RadialDivision, DAI_CAMERA_MODEL_RADIAL_DIVISION);

DAI_PIN(dai::LengthUnit::METER, DAI_LENGTH_METER);
DAI_PIN(dai::LengthUnit::CENTIMETER, DAI_LENGTH_CENTIMETER);
DAI_PIN(dai::LengthUnit::MILLIMETER, DAI_LENGTH_MILLIMETER);
DAI_PIN(dai::LengthUnit::INCH, DAI_LENGTH_INCH);
DAI_PIN(dai::LengthUnit::FOOT, DAI_LENGTH_FOOT);
DAI_PIN(dai::LengthUnit::CUSTOM, DAI_LENGTH_CUSTOM);

DAI_PIN(dai::IMUSensor::ACCELEROMETER_RAW, DAI_IMU_ACCELEROMETER_RAW);
DAI_PIN(dai::IMUSensor::ACCELEROMETER_CALIBRATED, DAI_IMU_ACCELEROMETER_CALIBRATED);
DAI_PIN(dai::IMUSensor::GYROSCOPE_RAW, DAI_IMU_GYROSCOPE_RAW);
DAI_PIN(dai::IMUSensor::MAGNETOMETER_RAW, DAI_IMU_MAGNETOMETER_RAW);
DAI_PIN(dai::IMUSensor::ROTATION_VECTOR, DAI_IMU_ROTATION_VECTOR);

DAI_PIN(dai::IMUReport::Accuracy::UNRELIABLE, DAI_IMU_ACCURACY_UNRELIABLE);
DAI_PIN(dai::IMUReport::Accuracy::LOW, DAI_IMU_ACCURACY_LOW);
DAI_PIN(dai::IMUReport::Accuracy::MEDIUM, DAI_IMU_ACCURACY_MEDIUM);
DAI_PIN(dai::IMUReport::Accuracy::HIGH, DAI_IMU_ACCURACY_HIGH);

DAI_PIN(dai::VideoEncoderProperties::Profile::H264_BASELINE, DAI_VENC_H264_BASELINE);
DAI_PIN(dai::VideoEncoderProperties::Profile::H264_HIGH, DAI_VENC_H264_HIGH);
DAI_PIN(dai::VideoEncoderProperties::Profile::H264_MAIN, DAI_VENC_H264_MAIN);
DAI_PIN(dai::VideoEncoderProperties::Profile::H265_MAIN, DAI_VENC_H265_MAIN);
DAI_PIN(dai::VideoEncoderProperties::Profile::MJPEG, DAI_VENC_MJPEG);
DAI_PIN(dai::VideoEncoderProperties::RateControlMode::CBR, DAI_VENC_RC_CBR);
DAI_PIN(dai::VideoEncoderProperties::RateControlMode::VBR, DAI_VENC_RC_VBR);

DAI_PIN(dai::node::StereoDepth::PresetMode::FAST_ACCURACY, DAI_STEREO_PRESET_FAST_ACCURACY);
DAI_PIN(dai::node::StereoDepth::PresetMode::FAST_DENSITY, DAI_STEREO_PRESET_FAST_DENSITY);
DAI_PIN(dai::node::StereoDepth::PresetMode::DEFAULT, DAI_STEREO_PRESET_DEFAULT);
DAI_PIN(dai::node::StereoDepth::PresetMode::FACE, DAI_STEREO_PRESET_FACE);
DAI_PIN(dai::node::StereoDepth::PresetMode::HIGH_DETAIL, DAI_STEREO_PRESET_HIGH_DETAIL);
DAI_PIN(dai::node::StereoDepth::PresetMode::ROBOTICS, DAI_STEREO_PRESET_ROBOTICS);
DAI_PIN(dai::node::StereoDepth::PresetMode::DENSITY, DAI_STEREO_PRESET_DENSITY);
DAI_PIN(dai::node::StereoDepth::PresetMode::ACCURACY, DAI_STEREO_PRESET_ACCURACY);

DAI_PIN(X_LINK_ANY_STATE, DAI_XLINK_STATE_ANY);
DAI_PIN(X_LINK_BOOTED, DAI_XLINK_STATE_BOOTED);
DAI_PIN(X_LINK_UNBOOTED, DAI_XLINK_STATE_UNBOOTED);
DAI_PIN(X_LINK_BOOTLOADER, DAI_XLINK_STATE_BOOTLOADER);
DAI_PIN(X_LINK_FLASH_BOOTED, DAI_XLINK_STATE_FLASH_BOOTED);

DAI_PIN(X_LINK_USB_VSC, DAI_XLINK_PROTOCOL_USB_VSC);
DAI_PIN(X_LINK_USB_CDC, DAI_XLINK_PROTOCOL_USB_CDC);
DAI_PIN(X_LINK_PCIE, DAI_XLINK_PROTOCOL_PCIE);
DAI_PIN(X_LINK_IPC, DAI_XLINK_PROTOCOL_IPC);
DAI_PIN(X_LINK_TCP_IP, DAI_XLINK_PROTOCOL_TCP_IP);
DAI_PIN(X_LINK_LOCAL_SHDMEM, DAI_XLINK_PROTOCOL_LOCAL_SHDMEM);
DAI_PIN(X_LINK_TCP_IP_OR_LOCAL_SHDMEM, DAI_XLINK_PROTOCOL_TCP_IP_OR_LOCAL_SHDMEM);
DAI_PIN(X_LINK_USB_EP, DAI_XLINK_PROTOCOL_USB_EP);
DAI_PIN(X_LINK_ANY_PROTOCOL, DAI_XLINK_PROTOCOL_ANY);
DAI_PIN(X_LINK_ANY_PLATFORM, DAI_XLINK_PLATFORM_ANY);
DAI_PIN(X_LINK_MYRIAD_2, DAI_XLINK_PLATFORM_MYRIAD_2);
DAI_PIN(X_LINK_MYRIAD_X, DAI_XLINK_PLATFORM_MYRIAD_X);
DAI_PIN(X_LINK_RVC3, DAI_XLINK_PLATFORM_RVC3);
DAI_PIN(X_LINK_RVC4, DAI_XLINK_PLATFORM_RVC4);
DAI_PIN(dai::EncodedFrame::Profile::JPEG, DAI_ENC_PROFILE_JPEG);
DAI_PIN(dai::EncodedFrame::Profile::AVC, DAI_ENC_PROFILE_AVC);
DAI_PIN(dai::EncodedFrame::Profile::HEVC, DAI_ENC_PROFILE_HEVC);
DAI_PIN(dai::EncodedFrame::FrameType::I, DAI_ENC_FRAME_I);
DAI_PIN(dai::EncodedFrame::FrameType::P, DAI_ENC_FRAME_P);
DAI_PIN(dai::EncodedFrame::FrameType::B, DAI_ENC_FRAME_B);
DAI_PIN(dai::EncodedFrame::FrameType::Unknown, DAI_ENC_FRAME_UNKNOWN);

DAI_PIN(dai::Platform::RVC2, DAI_PLATFORM_RVC2);
DAI_PIN(dai::Platform::RVC3, DAI_PLATFORM_RVC3);
DAI_PIN(dai::Platform::RVC4, DAI_PLATFORM_RVC4);

// POD layout pins (mirrored by size_of tests in depthai-sys/src/lib.rs).
static_assert(sizeof(dai_device_info) == 152, "dai_device_info layout changed");
static_assert(sizeof(dai_buffer_info) == 48, "dai_buffer_info layout changed");
static_assert(sizeof(dai_img_frame_info) == 56, "dai_img_frame_info layout changed");
static_assert(sizeof(dai_imu_vec_report) == 56, "dai_imu_vec_report layout changed");
static_assert(sizeof(dai_imu_rotvec_report) == 64, "dai_imu_rotvec_report layout changed");
static_assert(sizeof(dai_imu_packet) == 232, "dai_imu_packet layout changed");
static_assert(sizeof(dai_encoded_frame_info) == 56, "dai_encoded_frame_info layout changed");
static_assert(sizeof(dai_tensor_info) == 164, "dai_tensor_info layout changed");
static_assert(sizeof(dai_img_detection) == 88, "dai_img_detection layout changed");

// ---------------------------------------------------------------------------
// Handle definitions
// ---------------------------------------------------------------------------
struct dai_device {
    std::shared_ptr<dai::Device> ptr;
};
struct dai_pipeline {
    std::shared_ptr<dai::Device> device;  // keeps the device alive as long as the pipeline
    std::unique_ptr<dai::Pipeline> ptr;
};
using SensorRes = std::optional<std::pair<uint32_t, uint32_t>>;
struct dai_node {
    std::shared_ptr<dai::Node> ptr;
};
struct dai_queue {
    std::shared_ptr<dai::MessageQueue> ptr;
};
struct dai_msg {
    std::shared_ptr<dai::ADatatype> ptr;
};
struct dai_input_queue {
    std::shared_ptr<dai::InputQueue> ptr;
};
struct dai_calib {
    dai::CalibrationHandler handler;
};
struct dai_bootloader {
    std::unique_ptr<dai::DeviceBootloader> ptr;
};
struct dai_nn_archive {
    dai::NNArchive archive;
};
// dai_output / dai_input are never defined: they are reinterpret_casts of the
// node-owned dai::Node::Output* / Input* pointers.
static dai::Node::Output* as_output(dai_output* o) {
    return reinterpret_cast<dai::Node::Output*>(o);
}
static dai::Node::Input* as_input(dai_input* i) {
    return reinterpret_cast<dai::Node::Input*>(i);
}
static dai_output* from_output(dai::Node::Output* o) {
    return reinterpret_cast<dai_output*>(o);
}
static dai_input* from_input(dai::Node::Input* i) {
    return reinterpret_cast<dai_input*>(i);
}

// ---------------------------------------------------------------------------
// Error plumbing
// ---------------------------------------------------------------------------
// NOTE for DAI_GUARD bodies: braces and angle brackets do NOT protect macro
// arguments from comma splitting — only parentheses do. Keep top-level commas
// (aggregate init `{a, b}`, `std::pair<A, B>`) out of guarded bodies: use a
// type alias or build the object in separate statements.
static thread_local std::string g_err;

static void set_err(const std::string& m) {
    g_err = m;
}

#define DAI_GUARD(fn, body)                                        \
    try {                                                          \
        body                                                       \
    } catch(const std::exception& e) {                             \
        set_err(std::string(#fn) + ": " + e.what());               \
        return DAI_ERR;                                            \
    } catch(...) {                                                 \
        set_err(std::string(#fn) + ": unknown C++ exception");     \
        return DAI_ERR;                                            \
    }

#define DAI_REQUIRE(cond, msg) \
    if(!(cond)) {              \
        set_err(msg);          \
        return DAI_ERR;        \
    }

// Graph-configuration lock. depthai-core's Node / Node::Output / Node::Input
// have NO internal synchronisation (plain vector push_back in link(), plain
// member writes in every setter, PipelineImpl::outputQueues push in
// createOutputQueue). The safe Rust crate marks node and port handles Sync on the
// strength of THIS mutex: every entry point that touches a node, a port, or
// builds/starts the pipeline takes it. Configuration is a pre-start, one-time
// activity, so a single global lock costs nothing.
static std::mutex g_graph_mutex;
#define DAI_LOCK_GRAPH std::lock_guard<std::mutex> dai_graph_lock_(g_graph_mutex)

static char* dup_string(const std::string& s) {
    char* out = static_cast<char*>(std::malloc(s.size() + 1));
    if(!out) throw std::bad_alloc();
    std::memcpy(out, s.c_str(), s.size() + 1);
    return out;
}

/// The shim's "fps <= 0 selects the C++ default" sentinel.
static std::optional<float> opt_fps(float fps) {
    std::optional<float> f;
    if(fps > 0.0f) f = fps;
    return f;
}

/// Newline-joined string list (the shim's list-out convention).
template <class Range>
static std::string join_lines(const Range& items) {
    std::string joined;
    for(const auto& item : items) {
        if(!joined.empty()) joined += '\n';
        joined += item;
    }
    return joined;
}

static void copy_fixed(char* dst, size_t cap, const std::string& s) {
    const size_t n = std::min(cap - 1, s.size());
    std::memcpy(dst, s.data(), n);
    dst[n] = '\0';
}

static dai::DeviceInfo to_dai_info(const dai_device_info& i) {
    return dai::DeviceInfo(std::string(i.name),
                           std::string(i.device_id),
                           (XLinkDeviceState_t)i.state,
                           (XLinkProtocol_t)i.protocol,
                           (XLinkPlatform_t)i.platform,
                           (XLinkError_t)i.status);
}

static void from_dai_info(const dai::DeviceInfo& i, dai_device_info& o) {
    std::memset(&o, 0, sizeof(o));
    copy_fixed(o.name, sizeof(o.name), i.name);
    copy_fixed(o.device_id, sizeof(o.device_id), i.deviceId);
    o.state = (int32_t)i.state;
    o.protocol = (int32_t)i.protocol;
    o.platform = (int32_t)i.platform;
    o.status = (int32_t)i.status;
}

static dai::NNModelDescription to_dai_desc(const dai_nn_model_description& d) {
    dai::NNModelDescription desc;
    if(d.model) desc.model = d.model;
    if(d.platform) desc.platform = d.platform;
    if(d.optimization_level) desc.optimizationLevel = d.optimization_level;
    if(d.compression_level) desc.compressionLevel = d.compression_level;
    if(d.snpe_version) desc.snpeVersion = d.snpe_version;
    if(d.model_precision_type) desc.modelPrecisionType = d.model_precision_type;
    return desc;
}

static const dai::NNArchive& archive_of(const dai_nn_archive* a) {
    if(!a) throw std::invalid_argument("null NNArchive handle");
    return a->archive;
}

static int64_t steady_ns(std::chrono::time_point<std::chrono::steady_clock, std::chrono::steady_clock::duration> t) {
    return std::chrono::duration_cast<std::chrono::nanoseconds>(t.time_since_epoch()).count();
}

template <class T>
static std::shared_ptr<T> node_as(dai_node* n, const char* what) {
    if(!n || !n->ptr) throw std::invalid_argument(std::string("null node handle (") + what + ")");
    auto typed = std::dynamic_pointer_cast<T>(n->ptr);
    if(!typed) throw std::invalid_argument(std::string("node is not a ") + what + " (it is " + n->ptr->getName() + ")");
    return typed;
}

template <class T>
static std::shared_ptr<T> msg_as(const dai_msg* m, const char* what) {
    if(!m || !m->ptr) throw std::invalid_argument(std::string("null message handle (") + what + ")");
    auto typed = std::dynamic_pointer_cast<T>(m->ptr);
    if(!typed)
        throw std::invalid_argument(std::string("message is not a ") + what + " (datatype "
                                    + std::to_string((int)m->ptr->getDatatype()) + ")");
    return typed;
}

static std::shared_ptr<dai::Device> device_of(dai_device* d) {
    if(!d || !d->ptr) throw std::invalid_argument("null device handle");
    return d->ptr;
}

static dai::Pipeline& pipeline_of(dai_pipeline* p) {
    if(!p || !p->ptr) throw std::invalid_argument("null pipeline handle");
    return *p->ptr;
}

static const dai::CalibrationHandler& calib_of(const dai_calib* c) {
    if(!c) throw std::invalid_argument("null calibration handle");
    return c->handler;
}

static std::shared_ptr<dai::MessageQueue> queue_of(dai_queue* q) {
    if(!q || !q->ptr) throw std::invalid_argument("null queue handle");
    return q->ptr;
}

static dai_msg* wrap_msg(std::shared_ptr<dai::ADatatype> p) {
    return new dai_msg{std::move(p)};
}

static dai_node* wrap_node(std::shared_ptr<dai::Node> p) {
    return new dai_node{std::move(p)};
}

// The IMUReport header shared by every report kind.
template <class Out>
static void fill_report_header(Out& out, const dai::IMUReport& r) {
    out.ts_sec = r.timestamp.sec;
    out.ts_nsec = r.timestamp.nsec;
    out.ts_device_sec = r.tsDevice.sec;
    out.ts_device_nsec = r.tsDevice.nsec;
    out.sequence = r.sequence;
    out.accuracy = (int32_t)r.accuracy;
}

static void fill_vec_report(dai_imu_vec_report& out, const dai::IMUReport& r, float x, float y, float z) {
    fill_report_header(out, r);
    out.x = x;
    out.y = y;
    out.z = z;
    out.pad_ = 0;
}

// NeuralNetwork and DetectionNetwork share the build() overloads (DetectionNetwork::Model
// is NeuralNetwork::Model); the templates are the one body behind both C mirrors.
template <class Net>
static int nn_build_camera(
    const char* what, dai_node* nn, dai_node* camera, const dai_nn_model_description* desc, float fps, int32_t resize_mode) {
    DAI_REQUIRE(desc, "null model description");
    DAI_GUARD(nn_build_camera, {
        DAI_LOCK_GRAPH;
        auto net = node_as<Net>(nn, what);
        auto cam = node_as<dai::node::Camera>(camera, "Camera");
        std::optional<dai::ImgResizeMode> resize;
        if(resize_mode >= 0) resize = (dai::ImgResizeMode)resize_mode;
        net->build(cam, to_dai_desc(*desc), opt_fps(fps), resize);
        return DAI_OK;
    })
}
template <class Net>
static int nn_build_output(const char* what, dai_node* nn, dai_output* output, const dai_nn_archive* archive) {
    DAI_REQUIRE(output, "null output port");
    DAI_GUARD(nn_build_output, {
        DAI_LOCK_GRAPH;
        node_as<Net>(nn, what)->build(*as_output(output), archive_of(archive));
        return DAI_OK;
    })
}

extern "C" {

// ---------------------------------------------------------------------------
// Global
// ---------------------------------------------------------------------------
const char* dai_last_error(void) {
    return g_err.c_str();
}
void dai_clear_last_error(void) {
    g_err.clear();
}
void dai_string_free(char* s) {
    std::free(s);
}
const char* dai_build_version(void) {
    return dai::build::VERSION;
}
int dai_steady_clock_now_ns(int64_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_steady_clock_now_ns, {
        *out = steady_ns(std::chrono::steady_clock::now());
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// Device
// ---------------------------------------------------------------------------
int dai_device_open(const char* name_or_id, int32_t max_usb_speed, dai_device** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_device_open, {
        const bool has_id = name_or_id != nullptr && name_or_id[0] != '\0';
        std::shared_ptr<dai::Device> dev;
        if(has_id && max_usb_speed >= 0) {
            dev = std::make_shared<dai::Device>(std::string(name_or_id), (dai::UsbSpeed)max_usb_speed);
        } else if(has_id) {
            dev = std::make_shared<dai::Device>(std::string(name_or_id));
        } else if(max_usb_speed >= 0) {
            dev = std::make_shared<dai::Device>((dai::UsbSpeed)max_usb_speed);
        } else {
            dev = std::make_shared<dai::Device>();
        }
        *out = new dai_device{std::move(dev)};
        return DAI_OK;
    })
}
int dai_device_open_info(const dai_device_info* info, int32_t max_usb_speed, dai_device** out) {
    DAI_REQUIRE(info && out, "null argument");
    DAI_GUARD(dai_device_open_info, {
        dai::DeviceInfo di = to_dai_info(*info);
        std::shared_ptr<dai::Device> dev;
        if(max_usb_speed >= 0) {
            dev = std::make_shared<dai::Device>(di, (dai::UsbSpeed)max_usb_speed);
        } else {
            dev = std::make_shared<dai::Device>(di);
        }
        *out = new dai_device{std::move(dev)};
        return DAI_OK;
    })
}
void dai_device_release(dai_device* d) {
    delete d;
}
int dai_device_close(dai_device* d) {
    DAI_GUARD(dai_device_close, {
        device_of(d)->close();
        return DAI_OK;
    })
}
int dai_device_is_closed(const dai_device* d, int* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_device_is_closed, {
        *out = device_of(const_cast<dai_device*>(d))->isClosed() ? 1 : 0;
        return DAI_OK;
    })
}
int dai_device_id(dai_device* d, char** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_device_id, {
        *out = dup_string(device_of(d)->getDeviceId());
        return DAI_OK;
    })
}
int dai_device_name(dai_device* d, char** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_device_name, {
        *out = dup_string(device_of(d)->getDeviceName());
        return DAI_OK;
    })
}
int dai_device_usb_speed(dai_device* d, int32_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_device_usb_speed, {
        *out = (int32_t)device_of(d)->getUsbSpeed();
        return DAI_OK;
    })
}
int dai_device_platform(dai_device* d, int32_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_device_platform, {
        *out = (int32_t)device_of(d)->getPlatform();
        return DAI_OK;
    })
}
int dai_device_connected_cameras(dai_device* d, int32_t* sockets, size_t cap, size_t* n) {
    DAI_REQUIRE(n, "null out pointer");
    DAI_GUARD(dai_device_connected_cameras, {
        auto cams = device_of(d)->getConnectedCameras();
        *n = cams.size();
        const size_t take = std::min(cap, cams.size());
        for(size_t i = 0; i < take; ++i) sockets[i] = (int32_t)cams[i];
        return DAI_OK;
    })
}
int dai_device_connected_imu(dai_device* d, char** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_device_connected_imu, {
        *out = dup_string(device_of(d)->getConnectedIMU());
        return DAI_OK;
    })
}
int dai_device_read_calibration(dai_device* d, dai_calib** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_device_read_calibration, {
        *out = new dai_calib{device_of(d)->readCalibration()};
        return DAI_OK;
    })
}
int dai_device_set_ir_laser_dot_projector_intensity(dai_device* d, float intensity, int32_t mask, int* out_ok) {
    DAI_REQUIRE(out_ok, "null out pointer");
    DAI_GUARD(dai_device_set_ir_laser_dot_projector_intensity, {
        *out_ok = device_of(d)->setIrLaserDotProjectorIntensity(intensity, mask < 0 ? -1 : (int)mask) ? 1 : 0;
        return DAI_OK;
    })
}
int dai_device_set_ir_flood_light_intensity(dai_device* d, float intensity, int32_t mask, int* out_ok) {
    DAI_REQUIRE(out_ok, "null out pointer");
    DAI_GUARD(dai_device_set_ir_flood_light_intensity, {
        *out_ok = device_of(d)->setIrFloodLightIntensity(intensity, mask < 0 ? -1 : (int)mask) ? 1 : 0;
        return DAI_OK;
    })
}
int dai_device_all_available(dai_device_info* out, size_t cap, size_t* n) {
    DAI_REQUIRE(n, "null out pointer");
    DAI_GUARD(dai_device_all_available, {
        auto infos = dai::Device::getAllAvailableDevices();
        *n = infos.size();
        const size_t take = std::min(cap, infos.size());
        for(size_t i = 0; i < take; ++i) from_dai_info(infos[i], out[i]);
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// Bootloader
// ---------------------------------------------------------------------------
int dai_bootloader_open(const dai_device_info* info, dai_bootloader** out) {
    DAI_REQUIRE(info && out, "null argument");
    DAI_GUARD(dai_bootloader_open, {
        dai::DeviceInfo di = to_dai_info(*info);
        *out = new dai_bootloader{std::make_unique<dai::DeviceBootloader>(di)};
        return DAI_OK;
    })
}
void dai_bootloader_release(dai_bootloader* b) {
    delete b;
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------
int dai_pipeline_new(dai_device* device, dai_pipeline** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_pipeline_new, {
        auto dev = device_of(device);
        auto* pl = new dai_pipeline;
        pl->device = dev;
        pl->ptr = std::make_unique<dai::Pipeline>(dev);
        *out = pl;
        return DAI_OK;
    })
}
int dai_pipeline_new_host_only(dai_pipeline** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_pipeline_new_host_only, {
        auto* pl = new dai_pipeline;
        pl->ptr = std::make_unique<dai::Pipeline>(false);
        *out = pl;
        return DAI_OK;
    })
}
void dai_pipeline_release(dai_pipeline* p) {
    if(!p) return;
    try {
        if(p->ptr && p->ptr->isRunning()) p->ptr->stop();
    } catch(...) {
    }
    delete p;
}
int dai_pipeline_build(dai_pipeline* p) {
    DAI_GUARD(dai_pipeline_build, {
        DAI_LOCK_GRAPH;
        pipeline_of(p).build();
        return DAI_OK;
    })
}
int dai_pipeline_start(dai_pipeline* p) {
    DAI_GUARD(dai_pipeline_start, {
        DAI_LOCK_GRAPH;
        pipeline_of(p).start();
        return DAI_OK;
    })
}
int dai_pipeline_stop(dai_pipeline* p) {
    DAI_GUARD(dai_pipeline_stop, {
        DAI_LOCK_GRAPH;
        pipeline_of(p).stop();
        return DAI_OK;
    })
}
int dai_pipeline_wait(dai_pipeline* p) {
    DAI_GUARD(dai_pipeline_wait, {
        pipeline_of(p).wait();
        return DAI_OK;
    })
}
int dai_pipeline_is_running(const dai_pipeline* p, int* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_pipeline_is_running, {
        *out = pipeline_of(const_cast<dai_pipeline*>(p)).isRunning() ? 1 : 0;
        return DAI_OK;
    })
}
int dai_pipeline_is_built(const dai_pipeline* p, int* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_pipeline_is_built, {
        *out = pipeline_of(const_cast<dai_pipeline*>(p)).isBuilt() ? 1 : 0;
        return DAI_OK;
    })
}
int dai_pipeline_remove(dai_pipeline* p, dai_node* n) {
    DAI_GUARD(dai_pipeline_remove, {
        DAI_LOCK_GRAPH;
        DAI_REQUIRE(n && n->ptr, "null node handle");
        pipeline_of(p).remove(n->ptr);
        return DAI_OK;
    })
}
#define DAI_CREATE_NODE(fn, NodeType)                              \
    int fn(dai_pipeline* p, dai_node** out) {                      \
        DAI_REQUIRE(out, "null out pointer");                      \
        DAI_GUARD(fn, {                                            \
            DAI_LOCK_GRAPH;                                        \
            *out = wrap_node(pipeline_of(p).create<NodeType>());   \
            return DAI_OK;                                         \
        })                                                         \
    }
DAI_CREATE_NODE(dai_pipeline_create_camera, dai::node::Camera)
DAI_CREATE_NODE(dai_pipeline_create_sync, dai::node::Sync)
DAI_CREATE_NODE(dai_pipeline_create_stereo_depth, dai::node::StereoDepth)
DAI_CREATE_NODE(dai_pipeline_create_video_encoder, dai::node::VideoEncoder)
DAI_CREATE_NODE(dai_pipeline_create_imu, dai::node::IMU)
DAI_CREATE_NODE(dai_pipeline_create_gate, dai::node::Gate)
DAI_CREATE_NODE(dai_pipeline_create_neural_network, dai::node::NeuralNetwork)
DAI_CREATE_NODE(dai_pipeline_create_detection_network, dai::node::DetectionNetwork)

// ---------------------------------------------------------------------------
// Node (common)
// ---------------------------------------------------------------------------
void dai_node_release(dai_node* n) {
    delete n;
}
int dai_node_id(const dai_node* n, int64_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_node_id, {
        DAI_REQUIRE(n && n->ptr, "null node handle");
        *out = (int64_t)n->ptr->id;
        return DAI_OK;
    })
}
int dai_node_type_name(const dai_node* n, const char** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_node_type_name, {
        DAI_REQUIRE(n && n->ptr, "null node handle");
        *out = n->ptr->getName();
        return DAI_OK;
    })
}
int dai_node_output_ref(dai_node* n, const char* group, const char* name, dai_output** out) {
    DAI_REQUIRE(out && name, "null argument");
    DAI_GUARD(dai_node_output_ref, {
        DAI_LOCK_GRAPH;
        DAI_REQUIRE(n && n->ptr, "null node handle");
        dai::Node::Output* o = n->ptr->getOutputRef(std::string(group ? group : ""), std::string(name));
        if(!o) return 0;
        *out = from_output(o);
        return 1;
    })
}
int dai_node_input_ref(dai_node* n, const char* group, const char* name, dai_input** out) {
    DAI_REQUIRE(out && name, "null argument");
    DAI_GUARD(dai_node_input_ref, {
        DAI_LOCK_GRAPH;
        DAI_REQUIRE(n && n->ptr, "null node handle");
        dai::Node::Input* i = n->ptr->getInputRef(std::string(group ? group : ""), std::string(name));
        if(!i) return 0;
        *out = from_input(i);
        return 1;
    })
}
int dai_node_output_names(dai_node* n, char** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_node_output_names, {
        DAI_LOCK_GRAPH;
        DAI_REQUIRE(n && n->ptr, "null node handle");
        std::vector<std::string> names;
        for(dai::Node::Output* o : n->ptr->getOutputRefs()) names.push_back(o->getGroup() + "/" + o->getName());
        *out = dup_string(join_lines(names));
        return DAI_OK;
    })
}
int dai_node_input_names(dai_node* n, char** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_node_input_names, {
        DAI_LOCK_GRAPH;
        DAI_REQUIRE(n && n->ptr, "null node handle");
        std::vector<std::string> names;
        for(dai::Node::Input* i : n->ptr->getInputRefs()) names.push_back(i->getGroup() + "/" + i->getName());
        *out = dup_string(join_lines(names));
        return DAI_OK;
    })
}
int dai_output_name(dai_output* o, char** out) {
    DAI_REQUIRE(o && out, "null argument");
    DAI_GUARD(dai_output_name, {
        DAI_LOCK_GRAPH;
        *out = dup_string(as_output(o)->getName());
        return DAI_OK;
    })
}
int dai_output_link(dai_output* o, dai_input* i) {
    DAI_REQUIRE(o && i, "null argument");
    DAI_GUARD(dai_output_link, {
        DAI_LOCK_GRAPH;
        as_output(o)->link(*as_input(i));
        return DAI_OK;
    })
}
int dai_output_unlink(dai_output* o, dai_input* i) {
    DAI_REQUIRE(o && i, "null argument");
    DAI_GUARD(dai_output_unlink, {
        DAI_LOCK_GRAPH;
        as_output(o)->unlink(*as_input(i));
        return DAI_OK;
    })
}
int dai_output_create_queue(dai_output* o, uint32_t max_size, int blocking, dai_queue** out) {
    DAI_REQUIRE(o && out, "null argument");
    DAI_GUARD(dai_output_create_queue, {
        DAI_LOCK_GRAPH;
        *out = new dai_queue{as_output(o)->createOutputQueue(max_size, blocking != 0)};
        return DAI_OK;
    })
}
int dai_input_create_queue(dai_input* i, uint32_t max_size, int blocking, dai_input_queue** out) {
    DAI_REQUIRE(i && out, "null argument");
    DAI_GUARD(dai_input_create_queue, {
        DAI_LOCK_GRAPH;
        *out = new dai_input_queue{as_input(i)->createInputQueue(max_size, blocking != 0)};
        return DAI_OK;
    })
}
int dai_input_set_blocking(dai_input* i, int blocking) {
    DAI_REQUIRE(i, "null input");
    DAI_GUARD(dai_input_set_blocking, {
        DAI_LOCK_GRAPH;
        as_input(i)->setBlocking(blocking != 0);
        return DAI_OK;
    })
}
int dai_input_set_max_size(dai_input* i, uint32_t max_size) {
    DAI_REQUIRE(i, "null input");
    DAI_GUARD(dai_input_set_max_size, {
        DAI_LOCK_GRAPH;
        as_input(i)->setMaxSize(max_size);
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// Camera
// ---------------------------------------------------------------------------
int dai_camera_build(dai_node* cam, int32_t socket, int32_t sensor_w, int32_t sensor_h, float sensor_fps) {
    DAI_GUARD(dai_camera_build, {
        DAI_LOCK_GRAPH;
        auto c = node_as<dai::node::Camera>(cam, "Camera");
        SensorRes res;
        if(sensor_w > 0 && sensor_h > 0) res = std::make_pair((uint32_t)sensor_w, (uint32_t)sensor_h);
        std::optional<float> fps;
        if(sensor_fps > 0.0f) fps = sensor_fps;
        c->build((dai::CameraBoardSocket)socket, res, fps);
        return DAI_OK;
    })
}
int dai_camera_board_socket(const dai_node* cam, int32_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_camera_board_socket, {
        DAI_LOCK_GRAPH;
        auto c = node_as<dai::node::Camera>(const_cast<dai_node*>(cam), "Camera");
        *out = (int32_t)c->getBoardSocket();
        return DAI_OK;
    })
}
int dai_camera_request_output(dai_node* cam, uint32_t w, uint32_t h, int32_t type, int32_t resize_mode, float fps,
                              int32_t undistort, dai_output** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_camera_request_output, {
        DAI_LOCK_GRAPH;
        auto c = node_as<dai::node::Camera>(cam, "Camera");
        std::optional<dai::ImgFrame::Type> ty;
        if(type >= 0) ty = (dai::ImgFrame::Type)type;
        std::optional<float> f = opt_fps(fps);
        std::optional<bool> und;
        if(undistort >= 0) und = undistort != 0;
        dai::Node::Output* o = c->requestOutput(std::make_pair(w, h), ty, (dai::ImgResizeMode)resize_mode, f, und);
        DAI_REQUIRE(o, "Camera::requestOutput returned null");
        *out = from_output(o);
        return DAI_OK;
    })
}
int dai_camera_request_full_resolution_output(dai_node* cam, int32_t type, float fps, int use_highest_resolution,
                                              dai_output** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_camera_request_full_resolution_output, {
        DAI_LOCK_GRAPH;
        auto c = node_as<dai::node::Camera>(cam, "Camera");
        std::optional<dai::ImgFrame::Type> ty;
        if(type >= 0) ty = (dai::ImgFrame::Type)type;
        std::optional<float> f = opt_fps(fps);
        dai::Node::Output* o = c->requestFullResolutionOutput(ty, f, use_highest_resolution != 0);
        DAI_REQUIRE(o, "Camera::requestFullResolutionOutput returned null");
        *out = from_output(o);
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------
int dai_sync_input(dai_node* s, const char* key, dai_input** out) {
    DAI_REQUIRE(out && key, "null argument");
    DAI_GUARD(dai_sync_input, {
        DAI_LOCK_GRAPH;
        auto sync = node_as<dai::node::Sync>(s, "Sync");
        *out = from_input(&sync->inputs[std::string(key)]);
        return DAI_OK;
    })
}
int dai_sync_set_sync_threshold_ns(dai_node* s, int64_t ns) {
    DAI_GUARD(dai_sync_set_sync_threshold_ns, {
        DAI_LOCK_GRAPH;
        node_as<dai::node::Sync>(s, "Sync")->setSyncThreshold(std::chrono::nanoseconds(ns));
        return DAI_OK;
    })
}
int dai_sync_set_sync_attempts(dai_node* s, int32_t attempts) {
    DAI_GUARD(dai_sync_set_sync_attempts, {
        DAI_LOCK_GRAPH;
        node_as<dai::node::Sync>(s, "Sync")->setSyncAttempts((int)attempts);
        return DAI_OK;
    })
}
int dai_sync_set_run_on_host(dai_node* s, int run_on_host) {
    DAI_GUARD(dai_sync_set_run_on_host, {
        DAI_LOCK_GRAPH;
        node_as<dai::node::Sync>(s, "Sync")->setRunOnHost(run_on_host != 0);
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// StereoDepth
// ---------------------------------------------------------------------------
#define DAI_STEREO(fn, expr)                                            \
    DAI_GUARD(fn, {                                                     \
        DAI_LOCK_GRAPH;                                                 \
        auto sd = node_as<dai::node::StereoDepth>(s, "StereoDepth");    \
        expr;                                                           \
        return DAI_OK;                                                  \
    })
int dai_stereo_depth_set_default_profile_preset(dai_node* s, int32_t preset) {
    DAI_STEREO(dai_stereo_depth_set_default_profile_preset, sd->setDefaultProfilePreset((dai::node::StereoDepth::PresetMode)preset))
}
int dai_stereo_depth_set_left_right_check(dai_node* s, int enable) {
    DAI_STEREO(dai_stereo_depth_set_left_right_check, sd->setLeftRightCheck(enable != 0))
}
int dai_stereo_depth_set_subpixel(dai_node* s, int enable) {
    DAI_STEREO(dai_stereo_depth_set_subpixel, sd->setSubpixel(enable != 0))
}
int dai_stereo_depth_set_extended_disparity(dai_node* s, int enable) {
    DAI_STEREO(dai_stereo_depth_set_extended_disparity, sd->setExtendedDisparity(enable != 0))
}
int dai_stereo_depth_set_output_size(dai_node* s, int32_t w, int32_t h) {
    DAI_STEREO(dai_stereo_depth_set_output_size, sd->setOutputSize((int)w, (int)h))
}
int dai_stereo_depth_set_depth_align_socket(dai_node* s, int32_t socket) {
    DAI_STEREO(dai_stereo_depth_set_depth_align_socket, sd->setDepthAlign((dai::CameraBoardSocket)socket))
}
int dai_stereo_depth_set_confidence_threshold(dai_node* s, int32_t threshold) {
    DAI_STEREO(dai_stereo_depth_set_confidence_threshold, sd->initialConfig->setConfidenceThreshold((int)threshold))
}
int dai_stereo_depth_pp_set_spatial_filter_enable(dai_node* s, int enable) {
    DAI_STEREO(dai_stereo_depth_pp_set_spatial_filter_enable, sd->initialConfig->postProcessing.spatialFilter.enable = (enable != 0))
}
int dai_stereo_depth_pp_set_temporal_filter_enable(dai_node* s, int enable) {
    DAI_STEREO(dai_stereo_depth_pp_set_temporal_filter_enable, sd->initialConfig->postProcessing.temporalFilter.enable = (enable != 0))
}
int dai_stereo_depth_pp_set_speckle_filter_enable(dai_node* s, int enable) {
    DAI_STEREO(dai_stereo_depth_pp_set_speckle_filter_enable, sd->initialConfig->postProcessing.speckleFilter.enable = (enable != 0))
}
int dai_stereo_depth_pp_set_threshold_filter(dai_node* s, int32_t min_range, int32_t max_range) {
    DAI_STEREO(dai_stereo_depth_pp_set_threshold_filter, {
        sd->initialConfig->postProcessing.thresholdFilter.minRange = min_range;
        sd->initialConfig->postProcessing.thresholdFilter.maxRange = max_range;
    })
}
int dai_stereo_depth_pp_set_decimation_factor(dai_node* s, uint32_t factor) {
    DAI_STEREO(dai_stereo_depth_pp_set_decimation_factor, sd->initialConfig->postProcessing.decimationFilter.decimationFactor = factor)
}

// ---------------------------------------------------------------------------
// VideoEncoder
// ---------------------------------------------------------------------------
#define DAI_VENC(fn, expr)                                                  \
    DAI_GUARD(fn, {                                                         \
        DAI_LOCK_GRAPH;                                                     \
        auto ve = node_as<dai::node::VideoEncoder>(e, "VideoEncoder");      \
        expr;                                                               \
        return DAI_OK;                                                      \
    })
int dai_video_encoder_set_default_profile_preset(dai_node* e, float fps, int32_t profile) {
    DAI_VENC(dai_video_encoder_set_default_profile_preset, ve->setDefaultProfilePreset(fps, (dai::VideoEncoderProperties::Profile)profile))
}
int dai_video_encoder_set_keyframe_frequency(dai_node* e, int32_t freq) {
    DAI_VENC(dai_video_encoder_set_keyframe_frequency, ve->setKeyframeFrequency((int)freq))
}
int dai_video_encoder_set_bitrate_kbps(dai_node* e, int32_t kbps) {
    DAI_VENC(dai_video_encoder_set_bitrate_kbps, ve->setBitrateKbps((int)kbps))
}
int dai_video_encoder_set_bitrate(dai_node* e, int32_t bps) {
    DAI_VENC(dai_video_encoder_set_bitrate, ve->setBitrate((int)bps))
}
int dai_video_encoder_set_profile(dai_node* e, int32_t profile) {
    DAI_VENC(dai_video_encoder_set_profile, ve->setProfile((dai::VideoEncoderProperties::Profile)profile))
}
int dai_video_encoder_set_rate_control_mode(dai_node* e, int32_t mode) {
    DAI_VENC(dai_video_encoder_set_rate_control_mode, ve->setRateControlMode((dai::VideoEncoderProperties::RateControlMode)mode))
}
int dai_video_encoder_set_num_bframes(dai_node* e, int32_t n) {
    DAI_VENC(dai_video_encoder_set_num_bframes, ve->setNumBFrames((int)n))
}
int dai_video_encoder_set_quality(dai_node* e, int32_t quality) {
    DAI_VENC(dai_video_encoder_set_quality, ve->setQuality((int)quality))
}
int dai_video_encoder_set_lossless(dai_node* e, int lossless) {
    DAI_VENC(dai_video_encoder_set_lossless, ve->setLossless(lossless != 0))
}

// ---------------------------------------------------------------------------
// Gate
// ---------------------------------------------------------------------------
int dai_gate_set_run_on_host(dai_node* g, int run_on_host) {
    DAI_GUARD(dai_gate_set_run_on_host, {
        DAI_LOCK_GRAPH;
        node_as<dai::node::Gate>(g, "Gate")->setRunOnHost(run_on_host != 0);
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// InputQueue (host -> device)
// ---------------------------------------------------------------------------
void dai_input_queue_release(dai_input_queue* q) {
    delete q;
}
int dai_input_queue_send(dai_input_queue* q, const dai_msg* m) {
    DAI_GUARD(dai_input_queue_send, {
        DAI_REQUIRE(q && q->ptr, "null input queue handle");
        DAI_REQUIRE(m && m->ptr, "null message handle");
        q->ptr->send(m->ptr);
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// Model zoo + NNArchive
// ---------------------------------------------------------------------------
int dai_model_zoo_get(const dai_nn_model_description* desc, int use_cached, const char* cache_dir, const char* api_key,
                      char** out_path) {
    DAI_REQUIRE(desc && out_path, "null argument");
    DAI_GUARD(dai_model_zoo_get, {
        auto path = dai::getModelFromZoo(to_dai_desc(*desc), use_cached != 0, std::string(cache_dir ? cache_dir : ""),
                                         std::string(api_key ? api_key : ""), "none");
        *out_path = dup_string(path.string());
        return DAI_OK;
    })
}
int dai_nn_archive_open(const char* path, dai_nn_archive** out) {
    DAI_REQUIRE(path && out, "null argument");
    DAI_GUARD(dai_nn_archive_open, {
        *out = new dai_nn_archive{dai::NNArchive(std::filesystem::path(path))};
        return DAI_OK;
    })
}
void dai_nn_archive_release(dai_nn_archive* a) {
    delete a;
}
int dai_nn_archive_input_size(const dai_nn_archive* a, uint32_t index, uint32_t* w, uint32_t* h) {
    DAI_REQUIRE(w && h, "null out pointer");
    DAI_GUARD(dai_nn_archive_input_size, {
        auto size = archive_of(a).getInputSize(index);
        if(!size) return 0;
        *w = size->first;
        *h = size->second;
        return 1;
    })
}

// ---------------------------------------------------------------------------
// NeuralNetwork
// ---------------------------------------------------------------------------
#define DAI_NN(fn, expr)                                                    \
    DAI_GUARD(fn, {                                                         \
        DAI_LOCK_GRAPH;                                                     \
        auto net = node_as<dai::node::NeuralNetwork>(nn, "NeuralNetwork");  \
        expr;                                                               \
        return DAI_OK;                                                      \
    })
int dai_neural_network_build_camera(
    dai_node* nn, dai_node* camera, const dai_nn_model_description* desc, float fps, int32_t resize_mode) {
    return nn_build_camera<dai::node::NeuralNetwork>("NeuralNetwork", nn, camera, desc, fps, resize_mode);
}
int dai_neural_network_build_output(dai_node* nn, dai_output* output, const dai_nn_archive* archive) {
    return nn_build_output<dai::node::NeuralNetwork>("NeuralNetwork", nn, output, archive);
}
int dai_neural_network_set_nn_archive(dai_node* nn, const dai_nn_archive* archive) {
    DAI_NN(dai_neural_network_set_nn_archive, net->setNNArchive(archive_of(archive)))
}
int dai_neural_network_set_num_inference_threads(dai_node* nn, int32_t n) {
    DAI_NN(dai_neural_network_set_num_inference_threads, net->setNumInferenceThreads((int)n))
}
int dai_neural_network_set_num_pool_frames(dai_node* nn, int32_t n) {
    DAI_NN(dai_neural_network_set_num_pool_frames, net->setNumPoolFrames((int)n))
}

// ---------------------------------------------------------------------------
// DetectionNetwork
// ---------------------------------------------------------------------------
#define DAI_DN(fn, expr)                                                          \
    DAI_GUARD(fn, {                                                               \
        DAI_LOCK_GRAPH;                                                           \
        auto net = node_as<dai::node::DetectionNetwork>(dn, "DetectionNetwork");  \
        expr;                                                                     \
        return DAI_OK;                                                            \
    })
/* Its ports are references into the subnodes (not in getOutputRefs()), so each is a member read. */
#define DAI_DN_PORT(fn, member, wrap, T)                                                   \
    int fn(dai_node* dn, T** out) {                                                        \
        DAI_REQUIRE(out, "null out pointer");                                              \
        DAI_DN(fn, *out = wrap(&net->member))                                              \
    }
int dai_detection_network_build_camera(
    dai_node* dn, dai_node* camera, const dai_nn_model_description* desc, float fps, int32_t resize_mode) {
    return nn_build_camera<dai::node::DetectionNetwork>("DetectionNetwork", dn, camera, desc, fps, resize_mode);
}
int dai_detection_network_build_output(dai_node* dn, dai_output* output, const dai_nn_archive* archive) {
    return nn_build_output<dai::node::DetectionNetwork>("DetectionNetwork", dn, output, archive);
}
int dai_detection_network_set_confidence_threshold(dai_node* dn, float threshold) {
    DAI_DN(dai_detection_network_set_confidence_threshold, net->setConfidenceThreshold(threshold))
}
DAI_DN_PORT(dai_detection_network_input, input, from_input, dai_input)
DAI_DN_PORT(dai_detection_network_out, out, from_output, dai_output)
DAI_DN_PORT(dai_detection_network_out_network, outNetwork, from_output, dai_output)
DAI_DN_PORT(dai_detection_network_passthrough, passthrough, from_output, dai_output)
int dai_detection_network_classes(dai_node* dn, char** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_detection_network_classes, {
        auto classes = node_as<dai::node::DetectionNetwork>(dn, "DetectionNetwork")->getClasses();
        *out = dup_string(classes ? join_lines(*classes) : std::string());
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// IMU
// ---------------------------------------------------------------------------
int dai_imu_enable_sensor(dai_node* imu, int32_t sensor, uint32_t report_rate_hz) {
    DAI_GUARD(dai_imu_enable_sensor, {
        DAI_LOCK_GRAPH;
        node_as<dai::node::IMU>(imu, "IMU")->enableIMUSensor((dai::IMUSensor)sensor, report_rate_hz);
        return DAI_OK;
    })
}
int dai_imu_set_batch_report_threshold(dai_node* imu, int32_t n) {
    DAI_GUARD(dai_imu_set_batch_report_threshold, {
        DAI_LOCK_GRAPH;
        node_as<dai::node::IMU>(imu, "IMU")->setBatchReportThreshold(n);
        return DAI_OK;
    })
}
int dai_imu_set_max_batch_reports(dai_node* imu, int32_t n) {
    DAI_GUARD(dai_imu_set_max_batch_reports, {
        DAI_LOCK_GRAPH;
        node_as<dai::node::IMU>(imu, "IMU")->setMaxBatchReports(n);
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// Queue
// ---------------------------------------------------------------------------
void dai_queue_release(dai_queue* q) {
    delete q;
}
int dai_queue_try_get(dai_queue* q, dai_msg** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_queue_try_get, {
        auto m = queue_of(q)->tryGet();
        if(!m) return 0;
        *out = wrap_msg(std::move(m));
        return 1;
    })
}
int dai_queue_get(dai_queue* q, int64_t timeout_ns, dai_msg** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_queue_get, {
        auto qu = queue_of(q);
        std::shared_ptr<dai::ADatatype> m;
        // A timeout near INT64_MAX would overflow steady_clock::now() + timeout
        // inside depthai and return at once; treat it as "forever".
        if(timeout_ns < 0 || timeout_ns >= INT64_MAX / 2) {
            m = qu->get();
        } else {
            bool timedOut = false;
            m = qu->get(std::chrono::nanoseconds(timeout_ns), timedOut);
            if(timedOut || !m) return 0;
        }
        if(!m) return 0;
        *out = wrap_msg(std::move(m));
        return 1;
    })
}
int dai_queue_has(dai_queue* q, int* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_queue_has, {
        *out = queue_of(q)->has() ? 1 : 0;
        return DAI_OK;
    })
}
int dai_queue_size(dai_queue* q, uint32_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_queue_size, {
        *out = queue_of(q)->getSize();
        return DAI_OK;
    })
}
int dai_queue_close(dai_queue* q) {
    DAI_GUARD(dai_queue_close, {
        queue_of(q)->close();
        return DAI_OK;
    })
}
int dai_queue_is_closed(dai_queue* q, int* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_queue_is_closed, {
        *out = queue_of(q)->isClosed() ? 1 : 0;
        return DAI_OK;
    })
}
int dai_queue_set_blocking(dai_queue* q, int blocking) {
    DAI_GUARD(dai_queue_set_blocking, {
        queue_of(q)->setBlocking(blocking != 0);
        return DAI_OK;
    })
}
int dai_queue_set_max_size(dai_queue* q, uint32_t max_size) {
    DAI_GUARD(dai_queue_set_max_size, {
        queue_of(q)->setMaxSize(max_size);
        return DAI_OK;
    })
}
int dai_queue_name(dai_queue* q, char** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_queue_name, {
        *out = dup_string(queue_of(q)->getName());
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------
void dai_msg_release(dai_msg* m) {
    delete m;
}
int dai_msg_clone(const dai_msg* m, dai_msg** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_msg_clone, {
        DAI_REQUIRE(m && m->ptr, "null message handle");
        *out = wrap_msg(m->ptr);
        return DAI_OK;
    })
}
int dai_msg_datatype(const dai_msg* m, int32_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_msg_datatype, {
        DAI_REQUIRE(m && m->ptr, "null message handle");
        *out = (int32_t)m->ptr->getDatatype();
        return DAI_OK;
    })
}
int dai_buffer_get_info(const dai_msg* m, dai_buffer_info* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_buffer_get_info, {
        auto b = msg_as<dai::Buffer>(m, "Buffer");
        auto span = static_cast<const dai::Buffer&>(*b).getData();
        std::memset(out, 0, sizeof(*out));
        out->datatype = (int32_t)b->getDatatype();
        out->timestamp_ns = steady_ns(b->getTimestamp());
        out->timestamp_device_ns = steady_ns(b->getTimestampDevice());
        out->sequence_num = b->getSequenceNum();
        out->data = span.data();
        out->data_len = span.size();
        return DAI_OK;
    })
}
int dai_msg_data(const dai_msg* m, const uint8_t** ptr, size_t* len) {
    DAI_REQUIRE(ptr && len, "null out pointer");
    DAI_GUARD(dai_msg_data, {
        auto b = msg_as<dai::Buffer>(m, "Buffer");
        auto span = static_cast<const dai::Buffer&>(*b).getData();
        *ptr = span.data();
        *len = span.size();
        return DAI_OK;
    })
}
int dai_msg_timestamp_ns(const dai_msg* m, int64_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_msg_timestamp_ns, {
        *out = steady_ns(msg_as<dai::Buffer>(m, "Buffer")->getTimestamp());
        return DAI_OK;
    })
}
int dai_msg_timestamp_device_ns(const dai_msg* m, int64_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_msg_timestamp_device_ns, {
        *out = steady_ns(msg_as<dai::Buffer>(m, "Buffer")->getTimestampDevice());
        return DAI_OK;
    })
}
int dai_msg_sequence_num(const dai_msg* m, int64_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_msg_sequence_num, {
        *out = msg_as<dai::Buffer>(m, "Buffer")->getSequenceNum();
        return DAI_OK;
    })
}
int dai_img_frame_get_info(const dai_msg* m, dai_img_frame_info* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_img_frame_get_info, {
        auto f = msg_as<dai::ImgFrame>(m, "ImgFrame");
        std::memset(out, 0, sizeof(*out));
        out->width = f->getWidth();
        out->height = f->getHeight();
        out->stride = f->getStride();
        out->type = (int32_t)f->getType();
        out->instance_num = f->getInstanceNum();
        out->sequence_num = f->getSequenceNum();
        out->timestamp_ns = steady_ns(f->getTimestamp());
        out->timestamp_device_ns = steady_ns(f->getTimestampDevice());
        out->data_len = static_cast<const dai::ImgFrame&>(*f).getData().size();
        return DAI_OK;
    })
}
int dai_img_frame_plane_stride(const dai_msg* m, int32_t plane, uint32_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_img_frame_plane_stride, {
        *out = msg_as<dai::ImgFrame>(m, "ImgFrame")->getPlaneStride((int)plane);
        return DAI_OK;
    })
}
int dai_img_frame_plane_height(const dai_msg* m, uint32_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_img_frame_plane_height, {
        auto f = msg_as<dai::ImgFrame>(m, "ImgFrame");
        // getPlaneHeight() is planeStride / stride with no guard: a malformed frame
        // (width 0 or type NONE) would divide by zero.
        DAI_REQUIRE(f->getStride() != 0, "frame has zero stride");
        *out = f->getPlaneHeight();
        return DAI_OK;
    })
}
int dai_encoded_frame_get_info(const dai_msg* m, dai_encoded_frame_info* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_encoded_frame_get_info, {
        auto f = msg_as<dai::EncodedFrame>(m, "EncodedFrame");
        std::memset(out, 0, sizeof(*out));
        out->width = f->getWidth();
        out->height = f->getHeight();
        out->profile = (int32_t)f->getProfile();
        out->frame_type = (int32_t)f->getFrameType();
        out->quality = f->getQuality();
        out->bitrate = f->getBitrate();
        out->lossless = f->getLossless() ? 1 : 0;
        out->instance_num = f->getInstanceNum();
        out->sequence_num = f->getSequenceNum();
        out->timestamp_ns = steady_ns(f->getTimestamp());
        out->data_len = static_cast<const dai::EncodedFrame&>(*f).getData().size();
        return DAI_OK;
    })
}
int dai_imu_data_packets(const dai_msg* m, dai_imu_packet* out, size_t cap, size_t* n) {
    DAI_REQUIRE(n, "null out pointer");
    DAI_GUARD(dai_imu_data_packets, {
        auto d = msg_as<dai::IMUData>(m, "IMUData");
        *n = d->packets.size();
        const size_t take = std::min(cap, d->packets.size());
        for(size_t i = 0; i < take; ++i) {
            const dai::IMUPacket& p = d->packets[i];
            dai_imu_packet& o = out[i];
            std::memset(&o, 0, sizeof(o));
            fill_vec_report(o.accelerometer, p.acceleroMeter, p.acceleroMeter.x, p.acceleroMeter.y, p.acceleroMeter.z);
            fill_vec_report(o.gyroscope, p.gyroscope, p.gyroscope.x, p.gyroscope.y, p.gyroscope.z);
            fill_vec_report(o.magnetic_field, p.magneticField, p.magneticField.x, p.magneticField.y, p.magneticField.z);
            const auto& rv = p.rotationVector;
            fill_report_header(o.rotation_vector, rv);
            o.rotation_vector.i = rv.i;
            o.rotation_vector.j = rv.j;
            o.rotation_vector.k = rv.k;
            o.rotation_vector.real = rv.real;
            o.rotation_vector.accuracy_rad = rv.rotationVectorAccuracy;
        }
        return DAI_OK;
    })
}
static void copy_tensor_info(dai_tensor_info& o, const dai::TensorInfo& t) {
    o = dai_tensor_info{};
    copy_fixed(o.name, sizeof(o.name), t.name);
    o.datatype = (int32_t)t.dataType;
    o.order = (int32_t)t.order;
    o.num_dims = (uint32_t)std::min<size_t>(t.dims.size(), 8);
    for(uint32_t i = 0; i < o.num_dims; ++i) o.dims[i] = t.dims[i];
    o.num_strides = (uint32_t)std::min<size_t>(t.strides.size(), 8);
    for(uint32_t i = 0; i < o.num_strides; ++i) o.strides[i] = t.strides[i];
    o.offset = t.offset;
    o.quantization = t.quantization ? 1 : 0;
    o.qp_scale = t.qpScale;
    o.qp_zp = t.qpZp;
    o.elem_size = (uint32_t)t.getDataTypeSize();
}
int dai_nn_data_tensors(const dai_msg* m, dai_tensor_info* out, size_t cap, size_t* n) {
    DAI_REQUIRE(n, "null out pointer");
    DAI_GUARD(dai_nn_data_tensors, {
        const auto& tensors = msg_as<dai::NNData>(m, "NNData")->tensors;
        *n = tensors.size();
        const size_t take = std::min(cap, tensors.size());
        for(size_t i = 0; i < take; ++i) copy_tensor_info(out[i], tensors[i]);
        return DAI_OK;
    })
}
int dai_nn_data_tensor_info(const dai_msg* m, const char* name, dai_tensor_info* out) {
    DAI_REQUIRE(name && out, "null argument");
    DAI_GUARD(dai_nn_data_tensor_info, {
        auto info = msg_as<dai::NNData>(m, "NNData")->getTensorInfo(std::string(name));
        if(!info) return 0;
        copy_tensor_info(*out, *info);
        return 1;
    })
}
int dai_img_detections(const dai_msg* m, dai_img_detection* out, size_t cap, size_t* n) {
    DAI_REQUIRE(n, "null out pointer");
    DAI_GUARD(dai_img_detections, {
        const auto& dets = msg_as<dai::ImgDetections>(m, "ImgDetections")->detections;
        *n = dets.size();
        const size_t take = std::min(cap, dets.size());
        for(size_t i = 0; i < take; ++i) {
            const auto& d = dets[i];
            dai_img_detection& o = out[i];
            o = dai_img_detection{};
            o.label = d.label;
            o.confidence = d.confidence;
            o.xmin = d.xmin;
            o.ymin = d.ymin;
            o.xmax = d.xmax;
            o.ymax = d.ymax;
            copy_fixed(o.label_name, sizeof(o.label_name), d.labelName);
        }
        return DAI_OK;
    })
}
int dai_gate_control_new(int open, int32_t num_messages, int32_t fps, dai_msg** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_gate_control_new, {
        *out = wrap_msg(std::make_shared<dai::GateControl>(open != 0, (int)num_messages, (int)fps));
        return DAI_OK;
    })
}
int dai_msg_group_get(const dai_msg* g, const char* name, dai_msg** out) {
    DAI_REQUIRE(out && name, "null argument");
    DAI_GUARD(dai_msg_group_get, {
        auto grp = msg_as<dai::MessageGroup>(g, "MessageGroup");
        // MessageGroup::get() uses map operator[] and would insert a null entry
        // for an unknown name; probe the (public) map instead.
        auto it = grp->group.find(std::string(name));
        if(it == grp->group.end() || !it->second) return 0;
        *out = wrap_msg(it->second);
        return 1;
    })
}
int dai_msg_group_num_messages(const dai_msg* g, int64_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_msg_group_num_messages, {
        *out = msg_as<dai::MessageGroup>(g, "MessageGroup")->getNumMessages();
        return DAI_OK;
    })
}
int dai_msg_group_names(const dai_msg* g, char** out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_msg_group_names, {
        std::string joined;
        for(const auto& n : msg_as<dai::MessageGroup>(g, "MessageGroup")->getMessageNames()) {
            if(!joined.empty()) joined += '\n';
            joined += n;
        }
        *out = dup_string(joined);
        return DAI_OK;
    })
}
int dai_msg_group_is_synced(const dai_msg* g, int64_t threshold_ns, int* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_msg_group_is_synced, {
        *out = msg_as<dai::MessageGroup>(g, "MessageGroup")->isSynced(threshold_ns) ? 1 : 0;
        return DAI_OK;
    })
}
int dai_msg_group_interval_ns(const dai_msg* g, int64_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_msg_group_interval_ns, {
        *out = msg_as<dai::MessageGroup>(g, "MessageGroup")->getIntervalNs();
        return DAI_OK;
    })
}

// ---------------------------------------------------------------------------
// Calibration
// ---------------------------------------------------------------------------
static void copy_matrix(const std::vector<std::vector<float>>& m, size_t rows, size_t cols, float* out, const char* what) {
    if(m.size() != rows) throw std::runtime_error(std::string(what) + ": expected " + std::to_string(rows) + " rows, got " + std::to_string(m.size()));
    for(size_t i = 0; i < rows; ++i) {
        if(m[i].size() != cols)
            throw std::runtime_error(std::string(what) + ": expected " + std::to_string(cols) + " cols, got " + std::to_string(m[i].size()));
        for(size_t j = 0; j < cols; ++j) out[i * cols + j] = m[i][j];
    }
}
void dai_calib_release(dai_calib* c) {
    delete c;
}
int dai_calib_camera_intrinsics(const dai_calib* c, int32_t socket, int32_t w, int32_t h, float out_k[9]) {
    DAI_REQUIRE(out_k, "null out pointer");
    DAI_GUARD(dai_calib_camera_intrinsics, {
        auto k = calib_of(c).getCameraIntrinsics((dai::CameraBoardSocket)socket, (int)w, (int)h);
        copy_matrix(k, 3, 3, out_k, "getCameraIntrinsics");
        return DAI_OK;
    })
}
int dai_calib_distortion_coefficients(const dai_calib* c, int32_t socket, float* out, size_t cap, size_t* n) {
    DAI_REQUIRE(n, "null out pointer");
    DAI_GUARD(dai_calib_distortion_coefficients, {
        auto d = calib_of(c).getDistortionCoefficients((dai::CameraBoardSocket)socket);
        *n = d.size();
        const size_t take = std::min(cap, d.size());
        for(size_t i = 0; i < take; ++i) out[i] = d[i];
        return DAI_OK;
    })
}
int dai_calib_distortion_model(const dai_calib* c, int32_t socket, int32_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_calib_distortion_model, {
        *out = (int32_t)calib_of(c).getDistortionModel((dai::CameraBoardSocket)socket);
        return DAI_OK;
    })
}
int dai_calib_camera_extrinsics(const dai_calib* c, int32_t src, int32_t dst, int use_spec_translation, int32_t unit,
                                float out_t[16]) {
    DAI_REQUIRE(out_t, "null out pointer");
    DAI_GUARD(dai_calib_camera_extrinsics, {
        auto t = calib_of(c).getCameraExtrinsics((dai::CameraBoardSocket)src, (dai::CameraBoardSocket)dst,
                                                 use_spec_translation != 0, (dai::LengthUnit)unit);
        copy_matrix(t, 4, 4, out_t, "getCameraExtrinsics");
        return DAI_OK;
    })
}
int dai_calib_imu_to_camera_extrinsics(const dai_calib* c, int32_t socket, int use_spec_translation, int32_t unit,
                                       float out_t[16]) {
    DAI_REQUIRE(out_t, "null out pointer");
    DAI_GUARD(dai_calib_imu_to_camera_extrinsics, {
        auto t = calib_of(c).getImuToCameraExtrinsics((dai::CameraBoardSocket)socket, use_spec_translation != 0,
                                                      (dai::LengthUnit)unit);
        copy_matrix(t, 4, 4, out_t, "getImuToCameraExtrinsics");
        return DAI_OK;
    })
}
int dai_calib_camera_to_imu_extrinsics(const dai_calib* c, int32_t socket, int use_spec_translation, int32_t unit,
                                       float out_t[16]) {
    DAI_REQUIRE(out_t, "null out pointer");
    DAI_GUARD(dai_calib_camera_to_imu_extrinsics, {
        auto t = calib_of(c).getCameraToImuExtrinsics((dai::CameraBoardSocket)socket, use_spec_translation != 0,
                                                      (dai::LengthUnit)unit);
        copy_matrix(t, 4, 4, out_t, "getCameraToImuExtrinsics");
        return DAI_OK;
    })
}
int dai_calib_baseline_distance(const dai_calib* c, int32_t cam1, int32_t cam2, int use_spec_translation, int32_t unit,
                                float* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_calib_baseline_distance, {
        *out = calib_of(c).getBaselineDistance((dai::CameraBoardSocket)cam1, (dai::CameraBoardSocket)cam2,
                                               use_spec_translation != 0, (dai::LengthUnit)unit);
        return DAI_OK;
    })
}
int dai_calib_stereo_left_socket(const dai_calib* c, int32_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_calib_stereo_left_socket, {
        *out = (int32_t)calib_of(c).getStereoLeftCameraId();
        return DAI_OK;
    })
}
int dai_calib_stereo_right_socket(const dai_calib* c, int32_t* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_calib_stereo_right_socket, {
        *out = (int32_t)calib_of(c).getStereoRightCameraId();
        return DAI_OK;
    })
}
int dai_calib_fov(const dai_calib* c, int32_t socket, int use_spec, float* out) {
    DAI_REQUIRE(out, "null out pointer");
    DAI_GUARD(dai_calib_fov, {
        *out = calib_of(c).getFov((dai::CameraBoardSocket)socket, use_spec != 0);
        return DAI_OK;
    })
}

}  // extern "C"
