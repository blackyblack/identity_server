-module(punish_handler).

-export([init/2]).

init(#{method := <<"POST">>, bindings := #{user := User}} = Req, Opts) ->
    {ok, JsonData, State} = cowboy_req:read_body(Req),
    #{
        <<"signature">> := Signature,
        <<"nonce">> := Nonce,
        <<"signer">> := PublicKey,
        <<"idt">> := Balance,
        <<"proof_id">> := ProofId
    } = json:decode(JsonData),
    RefMessage = identity_signature:punish_signature_message(User, Nonce, Balance, ProofId),
    % same verification as for proofs
    ok = identity_signature:verify_and_consume_signature(proof, Signature, PublicKey, Nonce, RefMessage),
    {ok, Idt} = identity:punish(PublicKey, User, Balance, ProofId),
    Penalty = identity:penalty(User),
    PunishResp = json:encode(#{<<"from">> => PublicKey, <<"to">> => User, <<"idt">> => Idt, <<"penalty">> => Penalty}),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, PunishResp, State),
    {ok, Resp, Opts}.
