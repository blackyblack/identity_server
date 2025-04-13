-module(add_moderator_handler).

-export([init/2]).

init(#{method := <<"POST">>, bindings := #{user := User}} = Req, Opts) ->
    {ok, JsonData, State} = cowboy_req:read_body(Req),
    #{<<"signature">> := Signature, <<"nonce">> := Nonce, <<"signer">> := PublicKey} = json:decode(JsonData),
    RefMessage = identity_signature:moderators_signature_message(User, Nonce),
    ok = identity_signature:verify_and_consume_signature(moderators, Signature, PublicKey, Nonce, RefMessage),
    ok = moderators:add_moderator(PublicKey, User),
    ModeratorResp = json:encode(#{<<"from">> => PublicKey, <<"moderator">> => User}),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, ModeratorResp, State),
    {ok, Resp, Opts}.
