import Foundation

public enum ConfigError: Error, Equatable {
    case invalidTOML(String)
    case lockfileNotFound
}
