-module(moderators).

-export([is_moderator/1, add_moderator/1, remove_moderator/1]).

is_moderator(User) ->
    ets:member(moderators, User).

% priviledged call - should be called by admin only
add_moderator(User) ->
    ets:insert(moderators, {User}).

% priviledged call - should be called by admin only
remove_moderator(User) ->
    ets:delete_object(moderators, {User}).
