-module(app_logger).

-include_lib("kernel/include/logger.hrl").

-export([debug/2, info/2, warn/2, error/2]).

-spec debug(Format :: io:format(), Args :: [term()]) -> ok.
debug(Format, Args) -> ?LOG_DEBUG(Format, Args, #{domain => [identity_server]}).

-spec info(Format :: io:format(), Args :: [term()]) -> ok.
info(Format, Args) -> ?LOG_INFO(Format, Args, #{domain => [identity_server]}).

-spec warn(Format :: io:format(), Args :: [term()]) -> ok.
warn(Format, Args) -> ?LOG_WARNING(Format, Args, #{domain => [identity_server]}).

-spec error(Format :: io:format(), Args :: [term()]) -> ok.
error(Format, Args) -> ?LOG_ERROR(Format, Args, #{domain => [identity_server]}).
