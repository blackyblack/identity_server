-module(is_moderator_handler).

-export([init/2]).

init(#{method := <<"GET">>, bindings := #{user := User}} = Req, Opts) ->
    ModeratorResp = json:encode(#{<<"is_moderator">> => moderators:is_moderator(User)}),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, ModeratorResp, Req),
    {ok, Resp, Opts}.
