-module(add_admin_handler).

-export([init/2]).

init(#{method := <<"POST">>, bindings := #{user := User}} = Req, Opts) ->
    {ok, JsonData, State} = cowboy_req:read_body(Req),
    #{<<"signature">> := Signature, <<"nonce">> := Nonce, <<"signer">> := PublicKey} = json:decode(JsonData),
    RefMessage = identity_signature:admins_signature_message(User, Nonce),
    ok = identity_signature:verify_and_consume_signature(admins, Signature, PublicKey, Nonce, RefMessage),
    ok = admins:add_admin(PublicKey, User),
    AdminResp = json:encode(#{<<"from">> => PublicKey, <<"admin">> => User}),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, AdminResp, State),
    {ok, Resp, Opts}.
