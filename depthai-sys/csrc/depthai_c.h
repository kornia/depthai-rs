/* depthai_c.h — a pure-C ABI over depthai-core v3 (C++).
 *
 * The contract between depthai-core and the `depthai-sys` Rust crate. Rules:
 *
 *   - Every handle is an opaque struct pointer. Refcounted objects (device, node,
 *     queue, message, calibration) are heap copies of a std::shared_ptr and are
 *     released with the matching dai_*_release(). `dai_output` / `dai_input` are
 *     NON-owning raw pointers into a node and stay valid while that node lives.
 *   - Every function returns int: DAI_OK (0) or DAI_ERR (-1). Poll-style
 *     functions return 1 (got one) / 0 (none, timed out, absent) / -1 (error).
 *     Out-parameters are written only on success.
 *   - Every C++ exception is caught and stored in a THREAD-LOCAL error string,
 *     readable via dai_last_error(). Read it on the same thread, right after the
 *     failing call.
 *   - Enums cross as int32_t. The DAI_* constants below are static_assert'ed
 *     against the dai:: enumerators in depthai_c.cpp, so a depthai-core bump that
 *     renumbers one fails to compile instead of silently misbehaving.
 *   - Structs that cross are our own PODs with fixed layout, static_assert'ed on
 *     size in C and in Rust (`#[repr(C)]` mirrors in depthai-sys/src/lib.rs).
 *   - No policy: no defaults for calibration units/spec translation, no clock
 *     conversion, no env vars. Callers (the safe `depthai` crate and its users)
 *     own all of that.
 *   - RAW: one C function <-> one depthai-core member, named after it
 *     (dai_<class>_<member>). C++ overloads and std::optional arguments collapse
 *     into one function with sentinels (NULL/"" string, negative number) that
 *     select the C++ default; a *_get_info struct is a plain copy of that
 *     object's getters, nothing derived. If depthai-core has no such member,
 *     this ABI does not invent one.
 */
#ifndef DEPTHAI_C_H
#define DEPTHAI_C_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------------- */
/* Return codes                                                               */
/* ------------------------------------------------------------------------- */
#define DAI_OK 0
#define DAI_ERR (-1)

/* ------------------------------------------------------------------------- */
/* Opaque handles                                                             */
/* ------------------------------------------------------------------------- */
typedef struct dai_device dai_device;         /* heap std::shared_ptr<dai::Device>       */
typedef struct dai_pipeline dai_pipeline;     /* heap dai::Pipeline (+ device ref)        */
typedef struct dai_node dai_node;             /* heap std::shared_ptr<dai::Node>         */
typedef struct dai_output dai_output;         /* == dai::Node::Output* (non-owning)      */
typedef struct dai_input dai_input;           /* == dai::Node::Input*  (non-owning)      */
typedef struct dai_queue dai_queue;           /* heap std::shared_ptr<dai::MessageQueue> */
typedef struct dai_msg dai_msg;               /* heap std::shared_ptr<dai::ADatatype>    */
typedef struct dai_calib dai_calib;           /* heap dai::CalibrationHandler (copy)     */
typedef struct dai_bootloader dai_bootloader; /* heap dai::DeviceBootloader              */

/* ------------------------------------------------------------------------- */
/* Enum constants (int32_t on the wire; verified by static_assert in .cpp)    */
/* ------------------------------------------------------------------------- */
/* dai::CameraBoardSocket */
enum {
    DAI_CAM_AUTO = -1,
    DAI_CAM_A = 0,
    DAI_CAM_B = 1,
    DAI_CAM_C = 2,
    DAI_CAM_D = 3,
    DAI_CAM_E = 4,
    DAI_CAM_F = 5,
    DAI_CAM_G = 6,
    DAI_CAM_H = 7,
};
/* dai::UsbSpeed */
enum {
    DAI_USB_UNKNOWN = 0,
    DAI_USB_LOW = 1,
    DAI_USB_FULL = 2,
    DAI_USB_HIGH = 3,
    DAI_USB_SUPER = 4,
    DAI_USB_SUPER_PLUS = 5,
};
/* dai::ImgResizeMode */
enum {
    DAI_RESIZE_CROP = 0,
    DAI_RESIZE_STRETCH = 1,
    DAI_RESIZE_LETTERBOX = 2,
};
/* dai::ImgFrame::Type (subset; other values pass through as-is) */
enum {
    DAI_IMG_YUV420P = 2,
    DAI_IMG_RGB888P = 7,
    DAI_IMG_BGR888P = 8,
    DAI_IMG_RGB888I = 9,
    DAI_IMG_BGR888I = 10,
    DAI_IMG_RAW16 = 14,
    DAI_IMG_RAW8 = 18,
    DAI_IMG_NV12 = 22,
    DAI_IMG_BITSTREAM = 24,
    DAI_IMG_GRAY8 = 30,
    DAI_IMG_NONE = 33,
};
/* dai::DatatypeEnum (subset) */
enum {
    DAI_DT_ADATATYPE = 0,
    DAI_DT_BUFFER = 1,
    DAI_DT_IMG_FRAME = 2,
    DAI_DT_ENCODED_FRAME = 3,
    DAI_DT_IMU_DATA = 19,
    DAI_DT_MESSAGE_GROUP = 28,
};
/* dai::CameraModel */
enum {
    DAI_CAMERA_MODEL_PERSPECTIVE = 0,
    DAI_CAMERA_MODEL_FISHEYE = 1,
    DAI_CAMERA_MODEL_EQUIRECTANGULAR = 2,
    DAI_CAMERA_MODEL_RADIAL_DIVISION = 3,
};
/* dai::LengthUnit */
enum {
    DAI_LENGTH_METER = 0,
    DAI_LENGTH_CENTIMETER = 1,
    DAI_LENGTH_MILLIMETER = 2,
    DAI_LENGTH_INCH = 3,
    DAI_LENGTH_FOOT = 4,
    DAI_LENGTH_CUSTOM = 5,
};
/* dai::IMUSensor (subset) */
enum {
    DAI_IMU_ACCELEROMETER_RAW = 0x14,
    DAI_IMU_ACCELEROMETER_CALIBRATED = 0x01,
    DAI_IMU_GYROSCOPE_RAW = 0x15,
    DAI_IMU_MAGNETOMETER_RAW = 0x16,
    DAI_IMU_ROTATION_VECTOR = 0x05,
};
/* dai::IMUReport::Accuracy */
enum {
    DAI_IMU_ACCURACY_UNRELIABLE = 0,
    DAI_IMU_ACCURACY_LOW = 1,
    DAI_IMU_ACCURACY_MEDIUM = 2,
    DAI_IMU_ACCURACY_HIGH = 3,
};
/* dai::VideoEncoderProperties::Profile */
enum {
    DAI_VENC_H264_BASELINE = 0,
    DAI_VENC_H264_HIGH = 1,
    DAI_VENC_H264_MAIN = 2,
    DAI_VENC_H265_MAIN = 3,
    DAI_VENC_MJPEG = 4,
};
/* dai::VideoEncoderProperties::RateControlMode */
enum {
    DAI_VENC_RC_CBR = 0,
    DAI_VENC_RC_VBR = 1,
};
/* dai::node::StereoDepth::PresetMode */
enum {
    DAI_STEREO_PRESET_FAST_ACCURACY = 0,
    DAI_STEREO_PRESET_FAST_DENSITY = 1,
    DAI_STEREO_PRESET_DEFAULT = 2,
    DAI_STEREO_PRESET_FACE = 3,
    DAI_STEREO_PRESET_HIGH_DETAIL = 4,
    DAI_STEREO_PRESET_ROBOTICS = 5,
    DAI_STEREO_PRESET_DENSITY = 6,
    DAI_STEREO_PRESET_ACCURACY = 7,
};
/* XLinkDeviceState_t */
enum {
    DAI_XLINK_STATE_ANY = 0,
    DAI_XLINK_STATE_BOOTED = 1,
    DAI_XLINK_STATE_UNBOOTED = 2,
    DAI_XLINK_STATE_BOOTLOADER = 3,
    DAI_XLINK_STATE_FLASH_BOOTED = 4,
};
/* XLinkProtocol_t */
enum {
    DAI_XLINK_PROTOCOL_USB_VSC = 0,
    DAI_XLINK_PROTOCOL_USB_CDC = 1,
    DAI_XLINK_PROTOCOL_PCIE = 2,
    DAI_XLINK_PROTOCOL_IPC = 3,
    DAI_XLINK_PROTOCOL_TCP_IP = 4,
    DAI_XLINK_PROTOCOL_LOCAL_SHDMEM = 5,
    DAI_XLINK_PROTOCOL_TCP_IP_OR_LOCAL_SHDMEM = 6,
    DAI_XLINK_PROTOCOL_USB_EP = 7,
    DAI_XLINK_PROTOCOL_ANY = 9,
};
/* XLinkPlatform_t */
enum {
    DAI_XLINK_PLATFORM_ANY = 0,
    DAI_XLINK_PLATFORM_MYRIAD_2 = 2450,
    DAI_XLINK_PLATFORM_MYRIAD_X = 2480,
    DAI_XLINK_PLATFORM_RVC3 = 3000,
    DAI_XLINK_PLATFORM_RVC4 = 4000,
};
/* dai::EncodedFrame::Profile */
enum {
    DAI_ENC_PROFILE_JPEG = 0,
    DAI_ENC_PROFILE_AVC = 1,
    DAI_ENC_PROFILE_HEVC = 2,
};
/* dai::EncodedFrame::FrameType */
enum {
    DAI_ENC_FRAME_I = 0,
    DAI_ENC_FRAME_P = 1,
    DAI_ENC_FRAME_B = 2,
    DAI_ENC_FRAME_UNKNOWN = 3,
};
/* dai::Platform */
enum {
    DAI_PLATFORM_RVC2 = 0,
    DAI_PLATFORM_RVC3 = 1,
    DAI_PLATFORM_RVC4 = 2,
};

/* ------------------------------------------------------------------------- */
/* PODs (sizes pinned on both sides)                                          */
/* ------------------------------------------------------------------------- */
/* One entry of dai::Device::getAllAvailableDevices(). Strings are NUL-terminated
 * and truncated to fit. */
typedef struct dai_device_info {
    char name[64];      /* IP / USB path / name */
    char device_id[64]; /* MxId */
    int32_t state;      /* XLinkDeviceState_t */
    int32_t protocol;   /* XLinkProtocol_t */
    int32_t platform;   /* XLinkPlatform_t */
    int32_t status;     /* XLinkError_t */
    int32_t reserved_[2];
} dai_device_info; /* sizeof == 152 */

/* The dai::ADatatype / dai::Buffer getters every message has, in one copy. The
 * data pointer stays valid while any handle to the message lives. */
typedef struct dai_buffer_info {
    int32_t datatype; /* dai::DatatypeEnum */
    uint32_t pad_;
    int64_t timestamp_ns;        /* getTimestamp(): host steady_clock, ns since its epoch */
    int64_t timestamp_device_ns; /* getTimestampDevice(): device clock, ns since boot */
    int64_t sequence_num;
    const uint8_t* data; /* getData().data() */
    size_t data_len;     /* getData().size() */
} dai_buffer_info; /* sizeof == 48 */

/* Everything about a dai::ImgFrame except its pixels. */
typedef struct dai_img_frame_info {
    uint32_t width;
    uint32_t height;
    uint32_t stride;        /* getStride(): passed through as depthai reports it (may be 0) */
    int32_t type;           /* dai::ImgFrame::Type */
    uint32_t instance_num;  /* getInstanceNum() (camera socket for camera frames) */
    uint32_t pad_;
    int64_t sequence_num;
    int64_t timestamp_ns;        /* getTimestamp(): host steady_clock, ns since its epoch */
    int64_t timestamp_device_ns; /* getTimestampDevice(): device clock, ns since boot */
    size_t data_len;             /* getData().size() */
} dai_img_frame_info; /* sizeof == 56 */

/* One dai::IMUReport with x/y/z (accelerometer, gyroscope, magnetometer).
 * Timestamps are the RAW sec/nsec pair the firmware wrote: a {0,0} pair is the
 * value-initialised "no report" hole, which callers may want to detect. */
typedef struct dai_imu_vec_report {
    int64_t ts_sec;
    int64_t ts_nsec;
    int64_t ts_device_sec;
    int64_t ts_device_nsec;
    int32_t sequence;
    int32_t accuracy; /* dai::IMUReport::Accuracy widened */
    float x;
    float y;
    float z;
    float pad_;
} dai_imu_vec_report; /* sizeof == 56 */

/* dai::IMUReportRotationVectorWAcc */
typedef struct dai_imu_rotvec_report {
    int64_t ts_sec;
    int64_t ts_nsec;
    int64_t ts_device_sec;
    int64_t ts_device_nsec;
    int32_t sequence;
    int32_t accuracy;
    float i;
    float j;
    float k;
    float real;
    float accuracy_rad;
    float pad_;
} dai_imu_rotvec_report; /* sizeof == 64 */

/* dai::IMUPacket: the four value-member reports, copied verbatim. */
typedef struct dai_imu_packet {
    dai_imu_vec_report accelerometer;
    dai_imu_vec_report gyroscope;
    dai_imu_vec_report magnetic_field;
    dai_imu_rotvec_report rotation_vector;
} dai_imu_packet; /* sizeof == 232 */

/* dai::EncodedFrame metadata. */
typedef struct dai_encoded_frame_info {
    uint32_t width;
    uint32_t height;
    int32_t profile;    /* dai::EncodedFrame::Profile: 0 JPEG, 1 AVC, 2 HEVC */
    int32_t frame_type; /* dai::EncodedFrame::FrameType: 0 I, 1 P, 2 B, 3 Unknown */
    uint32_t quality;
    uint32_t bitrate;
    int32_t lossless;
    uint32_t instance_num;
    int64_t sequence_num;
    int64_t timestamp_ns;
    size_t data_len;
} dai_encoded_frame_info; /* sizeof == 56 */

/* ------------------------------------------------------------------------- */
/* Global                                                                     */
/* ------------------------------------------------------------------------- */
/* Last error on the calling thread; never NULL ("" when none). */
const char* dai_last_error(void);
void dai_clear_last_error(void);
/* Free a string returned through a `char**` out-parameter. NULL is a no-op. */
void dai_string_free(char* s);
/* depthai-core's build version string (static storage, do not free). */
const char* dai_build_version(void);
/* std::chrono::steady_clock::now() in ns since its epoch — the SAME clock every
 * message timestamp below is expressed in. The one deliberate non-member here:
 * Rust cannot read that clock's raw value itself, and every timestamp this ABI
 * hands out is relative to it. */
int dai_steady_clock_now_ns(int64_t* out);

/* ------------------------------------------------------------------------- */
/* Device                                                                     */
/* ------------------------------------------------------------------------- */
/* Connect. `name_or_id` NULL/"" = first available; else an MxId, IP or name.
 * `max_usb_speed` < 0 = library default. */
int dai_device_open(const char* name_or_id, int32_t max_usb_speed, dai_device** out);
/* Device(const DeviceInfo&, UsbSpeed): connect to an enumerated device.
 * `max_usb_speed` < 0 = library default. */
int dai_device_open_info(const dai_device_info* info, int32_t max_usb_speed, dai_device** out);
/* Drop this reference. The last reference runs ~Device (which closes). */
void dai_device_release(dai_device* d);
int dai_device_close(dai_device* d);
int dai_device_is_closed(const dai_device* d, int* out);
int dai_device_id(dai_device* d, char** out);
int dai_device_name(dai_device* d, char** out);
int dai_device_usb_speed(dai_device* d, int32_t* out);
int dai_device_platform(dai_device* d, int32_t* out);
/* Writes up to `cap` sockets; `*n` is the TOTAL count (may exceed cap). */
int dai_device_connected_cameras(dai_device* d, int32_t* sockets, size_t cap, size_t* n);
/* Raw firmware string ("" or "NONE" when absent — interpretation is the caller's). */
int dai_device_connected_imu(dai_device* d, char** out);
int dai_device_read_calibration(dai_device* d, dai_calib** out);
/* `mask` < 0 = all projectors. `*out_ok` = the bool depthai returns. */
int dai_device_set_ir_laser_dot_projector_intensity(dai_device* d, float intensity, int32_t mask, int* out_ok);
int dai_device_set_ir_flood_light_intensity(dai_device* d, float intensity, int32_t mask, int* out_ok);
/* dai::Device::getAllAvailableDevices(). Writes up to `cap`; `*n` = total. */
int dai_device_all_available(dai_device_info* out, size_t cap, size_t* n);

/* ------------------------------------------------------------------------- */
/* Bootloader                                                                 */
/* ------------------------------------------------------------------------- */
/* dai::DeviceBootloader(info). Releasing it runs the destructor, which reboots a
 * device that was sitting in bootloader state. */
int dai_bootloader_open(const dai_device_info* info, dai_bootloader** out);
void dai_bootloader_release(dai_bootloader* b);

/* ------------------------------------------------------------------------- */
/* Pipeline                                                                   */
/* ------------------------------------------------------------------------- */
int dai_pipeline_new(dai_device* device, dai_pipeline** out);
/* dai::Pipeline(false): no device; for building/inspecting graphs offline. */
int dai_pipeline_new_host_only(dai_pipeline** out);
void dai_pipeline_release(dai_pipeline* p);
int dai_pipeline_build(dai_pipeline* p);
int dai_pipeline_start(dai_pipeline* p);
int dai_pipeline_stop(dai_pipeline* p);
int dai_pipeline_wait(dai_pipeline* p);
int dai_pipeline_is_running(const dai_pipeline* p, int* out);
int dai_pipeline_is_built(const dai_pipeline* p, int* out);
/* Pipeline::remove(node): detach a node (and its links) from an unstarted graph. */
int dai_pipeline_remove(dai_pipeline* p, dai_node* n);
int dai_pipeline_create_camera(dai_pipeline* p, dai_node** out);
int dai_pipeline_create_sync(dai_pipeline* p, dai_node** out);
int dai_pipeline_create_stereo_depth(dai_pipeline* p, dai_node** out);
int dai_pipeline_create_video_encoder(dai_pipeline* p, dai_node** out);
int dai_pipeline_create_imu(dai_pipeline* p, dai_node** out);

/* ------------------------------------------------------------------------- */
/* Node (common)                                                              */
/* ------------------------------------------------------------------------- */
void dai_node_release(dai_node* n);
int dai_node_id(const dai_node* n, int64_t* out);
/* Node::getName() — static storage, do not free. */
int dai_node_type_name(const dai_node* n, const char** out);
/* Node::getOutputRef(group, name) / getInputRef(group, name). `group` NULL or ""
 * = the default group. Returns 1 found / 0 absent / -1 error. */
int dai_node_output_ref(dai_node* n, const char* group, const char* name, dai_output** out);
int dai_node_input_ref(dai_node* n, const char* group, const char* name, dai_input** out);
/* Newline-joined "group/name" list of every output / input ref. */
int dai_node_output_names(dai_node* n, char** out);
int dai_node_input_names(dai_node* n, char** out);

int dai_output_name(dai_output* o, char** out);
/* depthai validates datatype compatibility and throws -> DAI_ERR. */
int dai_output_link(dai_output* o, dai_input* i);
int dai_output_unlink(dai_output* o, dai_input* i);
int dai_output_create_queue(dai_output* o, uint32_t max_size, int blocking, dai_queue** out);
int dai_input_set_blocking(dai_input* i, int blocking);
int dai_input_set_max_size(dai_input* i, uint32_t max_size);

/* ------------------------------------------------------------------------- */
/* Camera                                                                     */
/* ------------------------------------------------------------------------- */
/* Camera::build(socket, sensorResolution?, sensorFps?). `sensor_w/h` <= 0 and
 * `sensor_fps` <= 0 mean nullopt. */
int dai_camera_build(dai_node* cam, int32_t socket, int32_t sensor_w, int32_t sensor_h, float sensor_fps);
int dai_camera_board_socket(const dai_node* cam, int32_t* out);
/* Camera::requestOutput(size, type?, resize, fps?, undistort?). `type` < 0,
 * `fps` <= 0 and `undistort` < 0 mean nullopt. The returned output is owned by
 * the node. */
int dai_camera_request_output(dai_node* cam, uint32_t w, uint32_t h, int32_t type, int32_t resize_mode, float fps,
                              int32_t undistort, dai_output** out);
int dai_camera_request_full_resolution_output(dai_node* cam, int32_t type, float fps, int use_highest_resolution,
                                              dai_output** out);

/* ------------------------------------------------------------------------- */
/* Sync                                                                       */
/* ------------------------------------------------------------------------- */
/* Sync::inputs[key]: get-or-CREATE, like InputMap::operator[]. */
int dai_sync_input(dai_node* s, const char* key, dai_input** out);
int dai_sync_set_sync_threshold_ns(dai_node* s, int64_t ns);
int dai_sync_set_sync_attempts(dai_node* s, int32_t attempts);
int dai_sync_set_run_on_host(dai_node* s, int run_on_host);

/* ------------------------------------------------------------------------- */
/* StereoDepth                                                                */
/* ------------------------------------------------------------------------- */
int dai_stereo_depth_set_default_profile_preset(dai_node* s, int32_t preset);
int dai_stereo_depth_set_left_right_check(dai_node* s, int enable);
int dai_stereo_depth_set_subpixel(dai_node* s, int enable);
int dai_stereo_depth_set_extended_disparity(dai_node* s, int enable);
int dai_stereo_depth_set_output_size(dai_node* s, int32_t w, int32_t h);
int dai_stereo_depth_set_depth_align_socket(dai_node* s, int32_t socket);
int dai_stereo_depth_set_confidence_threshold(dai_node* s, int32_t threshold);
/* initialConfig->postProcessing.* */
int dai_stereo_depth_pp_set_spatial_filter_enable(dai_node* s, int enable);
int dai_stereo_depth_pp_set_temporal_filter_enable(dai_node* s, int enable);
int dai_stereo_depth_pp_set_speckle_filter_enable(dai_node* s, int enable);
int dai_stereo_depth_pp_set_threshold_filter(dai_node* s, int32_t min_range, int32_t max_range);
int dai_stereo_depth_pp_set_decimation_factor(dai_node* s, uint32_t factor);

/* ------------------------------------------------------------------------- */
/* VideoEncoder                                                               */
/* ------------------------------------------------------------------------- */
int dai_video_encoder_set_default_profile_preset(dai_node* e, float fps, int32_t profile);
int dai_video_encoder_set_keyframe_frequency(dai_node* e, int32_t freq);
int dai_video_encoder_set_bitrate_kbps(dai_node* e, int32_t kbps);
int dai_video_encoder_set_bitrate(dai_node* e, int32_t bps);
int dai_video_encoder_set_profile(dai_node* e, int32_t profile);
int dai_video_encoder_set_rate_control_mode(dai_node* e, int32_t mode);
int dai_video_encoder_set_num_bframes(dai_node* e, int32_t n);
int dai_video_encoder_set_quality(dai_node* e, int32_t quality);
int dai_video_encoder_set_lossless(dai_node* e, int lossless);

/* ------------------------------------------------------------------------- */
/* IMU                                                                        */
/* ------------------------------------------------------------------------- */
int dai_imu_enable_sensor(dai_node* imu, int32_t sensor, uint32_t report_rate_hz);
int dai_imu_set_batch_report_threshold(dai_node* imu, int32_t n);
int dai_imu_set_max_batch_reports(dai_node* imu, int32_t n);

/* ------------------------------------------------------------------------- */
/* Queue                                                                      */
/* ------------------------------------------------------------------------- */
/* Drops our reference only; the producing Output keeps the queue linked. */
void dai_queue_release(dai_queue* q);
/* Non-blocking pop: 1 got / 0 empty / -1. */
int dai_queue_try_get(dai_queue* q, dai_msg** out);
/* Blocking pop with timeout (`timeout_ns` < 0 = forever): 1 got / 0 timed out / -1
 * (including a closed queue). */
int dai_queue_get(dai_queue* q, int64_t timeout_ns, dai_msg** out);
int dai_queue_has(dai_queue* q, int* out);
int dai_queue_size(dai_queue* q, uint32_t* out);
int dai_queue_close(dai_queue* q);
int dai_queue_is_closed(dai_queue* q, int* out);
int dai_queue_set_blocking(dai_queue* q, int blocking);
int dai_queue_set_max_size(dai_queue* q, uint32_t max_size);
int dai_queue_name(dai_queue* q, char** out);

/* ------------------------------------------------------------------------- */
/* Messages                                                                   */
/* ------------------------------------------------------------------------- */
void dai_msg_release(dai_msg* m);
/* shared_ptr copy: refcount++. */
int dai_msg_clone(const dai_msg* m, dai_msg** out);
int dai_msg_datatype(const dai_msg* m, int32_t* out);
/* All Buffer-level getters at once (see dai_buffer_info). */
int dai_buffer_get_info(const dai_msg* m, dai_buffer_info* out);
/* Buffer-level accessors (valid for every message type). The data pointer stays
 * valid while ANY handle to this message lives. */
int dai_msg_data(const dai_msg* m, const uint8_t** ptr, size_t* len);
int dai_msg_timestamp_ns(const dai_msg* m, int64_t* out);
int dai_msg_timestamp_device_ns(const dai_msg* m, int64_t* out);
int dai_msg_sequence_num(const dai_msg* m, int64_t* out);
/* ImgFrame (DAI_ERR if the message is not one). */
int dai_img_frame_get_info(const dai_msg* m, dai_img_frame_info* out);
int dai_img_frame_plane_stride(const dai_msg* m, int32_t plane, uint32_t* out);
int dai_img_frame_plane_height(const dai_msg* m, uint32_t* out);
/* EncodedFrame */
int dai_encoded_frame_get_info(const dai_msg* m, dai_encoded_frame_info* out);
/* IMUData: writes up to `cap` packets; `*n` = total packet count. */
int dai_imu_data_packets(const dai_msg* m, dai_imu_packet* out, size_t cap, size_t* n);
/* MessageGroup */
int dai_msg_group_get(const dai_msg* g, const char* name, dai_msg** out); /* 1 / 0 absent / -1 */
int dai_msg_group_num_messages(const dai_msg* g, int64_t* out);
int dai_msg_group_names(const dai_msg* g, char** out); /* newline-joined */
int dai_msg_group_is_synced(const dai_msg* g, int64_t threshold_ns, int* out);
int dai_msg_group_interval_ns(const dai_msg* g, int64_t* out);

/* ------------------------------------------------------------------------- */
/* Calibration                                                                */
/* ------------------------------------------------------------------------- */
void dai_calib_release(dai_calib* c);
/* getCameraIntrinsics(socket, w, h): `w`/`h` -1 = native. Row-major 3x3. Errors
 * if depthai returns anything but 3x3. */
int dai_calib_camera_intrinsics(const dai_calib* c, int32_t socket, int32_t w, int32_t h, float out_k[9]);
/* Writes up to `cap`; `*n` = total. */
int dai_calib_distortion_coefficients(const dai_calib* c, int32_t socket, float* out, size_t cap, size_t* n);
int dai_calib_distortion_model(const dai_calib* c, int32_t socket, int32_t* out);
/* Row-major 4x4. `use_spec_translation` and `unit` are MANDATORY. */
int dai_calib_camera_extrinsics(const dai_calib* c, int32_t src, int32_t dst, int use_spec_translation, int32_t unit,
                                float out_t[16]);
int dai_calib_imu_to_camera_extrinsics(const dai_calib* c, int32_t socket, int use_spec_translation, int32_t unit,
                                       float out_t[16]);
int dai_calib_camera_to_imu_extrinsics(const dai_calib* c, int32_t socket, int use_spec_translation, int32_t unit,
                                       float out_t[16]);
int dai_calib_baseline_distance(const dai_calib* c, int32_t cam1, int32_t cam2, int use_spec_translation, int32_t unit,
                                float* out);
int dai_calib_stereo_left_socket(const dai_calib* c, int32_t* out);
int dai_calib_stereo_right_socket(const dai_calib* c, int32_t* out);
int dai_calib_fov(const dai_calib* c, int32_t socket, int use_spec, float* out);

#ifdef __cplusplus
}
#endif

#endif /* DEPTHAI_C_H */
