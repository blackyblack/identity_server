-module(moderators).

-export([is_moderator/1, moderators_list/0, add_moderator/2, remove_moderator/2, add_moderator_from_config/1]).

is_moderator(User) ->
    ets:member(moderators, User).

moderators_list() ->
    Moders = ets:match(moderators, {'$1'}),
    lists:map(fun([A]) -> A end, Moders).

add_moderator(Admin, User) ->
    case admins:is_admin(Admin) of
        true -> ets:insert(moderators, {User});
        _ -> {error, not_allowed}
    end.

remove_moderator(Admin, User) ->
    case admins:is_admin(Admin) of
        true -> ets:delete_object(moderators, {User});
        _ -> {error, not_allowed}
    end.

add_moderator_from_config(User) ->
    ets:insert(moderators, {User}).
