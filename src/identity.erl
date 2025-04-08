-module(identity).

-export([vouch/2, idt/1, set_idt_by_proof/3]).

-define(TOP_VOUCHERS_SIZE, 5).
-define(MAX_IDT_BY_PROOF, 50000).

%TODO: might require to migrate to gen_server to make IDT updates atomic

vouch(From, To) ->
    ets:insert(vouches, {{To, From}, erlang:monotonic_time()}),
    {ok, idt(To)}.

% IDT calculation requires iterating all way down the graph so it can take
% substantial time and can be affected by graph reorganizations during
% traversal
idt(User) ->
    {Idt, _} = idt_visited(User, sets:new()),
    Idt.

% Priviledged method. Should be called by admin or moderator after proof verification.
% Proof should be publicly verifiable, probably with ZK proof.
set_idt_by_proof(Moderator, User, Balance) ->
    maybe
        ok ?= case moderators:is_moderator(Moderator) of
            false -> {error, not_allowed};
            _ -> ok
        end,
        ok ?= case Balance > ?MAX_IDT_BY_PROOF of
            true -> {error, max_balance_exceeded};
            _ -> ok
        end,
        ets:insert(id_proofs, {User, Balance, erlang:monotonic_time()}),
        ok
    end.

idt_visited(User, Visited) ->
    case sets:is_element(User, Visited) of
        true -> {0, Visited};
        _ ->
            VisitedUser = sets:add_element(User, Visited),
            TopVouchers = top_vouchers(User, VisitedUser),
            BalanceByVouchers = user_idt_from_vouchers(TopVouchers),
            BalanceByProof = user_idt_from_proof(User),
            {BalanceByVouchers + BalanceByProof, VisitedUser}
    end.

% returns a list of top vouchers with corresponding IDT balances and time of vouch event, [{Voucher, Balance, Time}]
top_vouchers(User, Visited) ->
    % Contains everyone who vouched for User
    VouchersWithTime = ets:match(vouches, {{User, '$1'}, '$2'}),
    {VouchersWithBalance, _Visited} = lists:mapfoldl(
        fun([UserV, TimeV], VisitedV) ->
            {Balance, VisitedUserV} = idt_visited(UserV, VisitedV),
            {{UserV, Balance, TimeV}, VisitedUserV}
        end,
        Visited,
        VouchersWithTime),
    VouchersWithBalanceSorted = lists:reverse(lists:sort(fun({_, BalanceA, _}, {_, BalanceB, _}) -> BalanceA =< BalanceB end, VouchersWithBalance)),
    lists:sublist(VouchersWithBalanceSorted, ?TOP_VOUCHERS_SIZE).

user_idt_from_vouchers([]) ->
    0;

user_idt_from_vouchers(Vouchers) ->
    Idt = lists:foldl(fun({_UserV, Balance, _TimeV}, TotalBalance) -> TotalBalance + Balance * 0.1 end, 0, Vouchers),
    trunc(math:ceil(Idt)).

user_idt_from_proof(User) ->
    ProvedBalance = ets:match(id_proofs, {User, '$1', '$2'}),
    case ProvedBalance of
        [] -> 0;
        [[Balance, _Time]] -> Balance;
        % unreachable
        _Rest -> 0
    end.
