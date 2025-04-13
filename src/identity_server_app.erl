-module(identity_server_app).
-behaviour(application).
-export([start/2, stop/1]).

-include("./include/args.hrl").

-define(DEFAULT_PORT, 8000).

start(_StartType, _StartArgs) ->
    identity_server:setup(),
    {ok, _} = application:ensure_all_started(cowboy),
    Port = case os:getenv("PORT") of
        false -> ?DEFAULT_PORT;
        P -> list_to_integer(P)
    end,
    Dispatch = cowboy_router:compile([
		{'_', [
			{"/vouch/:user", vouch_handler, []},
            {"/idt/:user", idt_handler, []},
            {"/proof/:user", proof_handler, []},
            {"/is_moderator/:user", is_moderator_handler, []},
            {"/add_moderator/:user", add_moderator_handler, []},
            {"/remove_moderator/:user", remove_moderator_handler, []},
            {'_', notfound_handler, []}
		]}
	]),
	{ok, _Pid} = cowboy:start_clear(http, [{port, Port}], #{
		env => #{dispatch => Dispatch}
	}),
    moderators:add_moderator(<<"8pYfL3PjD6kDgCBj5EEwmRsjhMA57qd5h4HygteaZ25Y">>, <<"8pYfL3PjD6kDgCBj5EEwmRsjhMA57qd5h4HygteaZ25Y">>),
    app_logger:info("Identity server started at localhost:~p", [Port]),
    % cowboy does not stop without supervisor link
    identity_server_sup:start_link().

stop(_State) ->
    cowboy:stop_listener(http).
