import SwiftUI
import UniformTypeIdentifiers
import UIKit

enum DriverCameraMode: String, Identifiable {
    case photo
    case video

    var id: String { rawValue }

    var mediaType: String {
        switch self {
        case .photo:
            UTType.image.identifier
        case .video:
            UTType.movie.identifier
        }
    }
}

struct DriverCapturedMedia {
    let data: Data
    let name: String
    let mimeType: String
}

struct DriverCameraCaptureView: UIViewControllerRepresentable {
    let mode: DriverCameraMode
    let onResult: (Result<DriverCapturedMedia, Error>) -> Void
    let onCancel: () -> Void

    final class Coordinator: NSObject, UINavigationControllerDelegate, UIImagePickerControllerDelegate {
        let parent: DriverCameraCaptureView

        init(parent: DriverCameraCaptureView) {
            self.parent = parent
        }

        func imagePickerControllerDidCancel(_ picker: UIImagePickerController) {
            parent.onCancel()
            picker.dismiss(animated: true)
        }

        func imagePickerController(
            _ picker: UIImagePickerController,
            didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey: Any]
        ) {
            defer { picker.dismiss(animated: true) }

            do {
                switch parent.mode {
                case .photo:
                    guard let image = info[.originalImage] as? UIImage,
                          let data = image.jpegData(compressionQuality: 0.9)
                    else {
                        throw DriverCameraError.captureUnavailable
                    }
                    parent.onResult(
                        .success(
                            DriverCapturedMedia(
                                data: data,
                                name: "driver-photo-\(Self.timestamp()).jpg",
                                mimeType: "image/jpeg"
                            )
                        )
                    )
                case .video:
                    guard let url = info[.mediaURL] as? URL else {
                        throw DriverCameraError.captureUnavailable
                    }
                    parent.onResult(
                        .success(
                            DriverCapturedMedia(
                                data: try Data(contentsOf: url),
                                name: "driver-video-\(Self.timestamp()).mov",
                                mimeType: "video/quicktime"
                            )
                        )
                    )
                }
            } catch {
                parent.onResult(.failure(error))
            }
        }

        private static func timestamp() -> UInt64 {
            UInt64(max(0, Date().timeIntervalSince1970 * 1000))
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(parent: self)
    }

    func makeUIViewController(context: Context) -> UIImagePickerController {
        let picker = UIImagePickerController()
        picker.delegate = context.coordinator
        picker.sourceType = .camera
        picker.mediaTypes = [mode.mediaType]
        if mode == .video {
            picker.videoQuality = .typeMedium
        }
        return picker
    }

    func updateUIViewController(_ uiViewController: UIImagePickerController, context: Context) {}
}

enum DriverCameraError: Error {
    case captureUnavailable
}
