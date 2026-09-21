export type MovieStatic = {
  id: number;
  title: string;
  originalTitle: string | null;
  urlKey: string;
  duration: number | null;
  contentRating: string | null;
  ratingColor: string | null;
  genres: string[];
  distributor: string | null;
  poster: string | null;
  backdrop: string | null;
  trailer: string | null;
  tags: string[];
  isReexhibition: boolean;
  inPreSale: boolean;
};

export type Session = {
  id: string;
  movieId: number;
  time: string;
  price: number | null;
  room: string | null;
  format: string;
  audio: string | null;
  checkoutUrl: string | null;
};

export type DenormalizedSession = {
  time: string;
  sessionId: string;
  priceInteira: number | null;
  priceMeia: number | null;
  gratuito: boolean;
  room: string | null;
  format: string;
  audio: string | null;
};

export type DenormalizedMovie = MovieStatic & {
  name: string;
  sessions: DenormalizedSession[];
};

export type UpcomingItem = {
  id: number;
  title: string;
  originalTitle: string | null;
  contentRating: string | null;
  genres: string[];
  poster: string | null;
  inPreSale: boolean;
  formats: string[];
  priceFrom: number | null;
  firstDate: string;
  firstDateFormatted: string;
  firstDateDayOfWeek: string;
  siteURL: string | null;
};

export type SessionDayCache = {
  fetchedAt: string;
  items: Session[];
};

export type UpcomingCache = {
  fetchedAt: string;
  items: UpcomingItem[];
};

export type CacheData = {
  movies: Record<string, MovieStatic>;
  sessions: Record<string, Record<string, SessionDayCache>>;
  upcoming: Record<string, UpcomingCache>;
  moviesUpdatedAt: string | null;
};

export type NormalizedSessions = {
  movies: Record<string, MovieStatic>;
  sessions: Session[];
  date: string | null;
  fetchedAt: string;
};

export type Cinema = {
  id: string;
  name: string;
  label: string;
  url: string;
};

export type RatingsResult = {
  imdb: string | null;
  rottenTomatoes: string | null;
  tmdb: string | null;
};

export type CarouselType = 'hoje' | 'amanha' | 'proximos';

export type InlineKeyboardButton = {
  text: string;
  callback_data?: string;
  url?: string;
};

export type InlineKeyboardMarkup = {
  inline_keyboard: InlineKeyboardButton[][];
};

export type BotLike = {
  sendMessage(
    chatId: number | string,
    text: string,
    options?: Record<string, unknown>,
  ): Promise<{ message_id: number }>;
  sendPhoto(
    chatId: number | string,
    photo: string,
    options?: Record<string, unknown>,
  ): Promise<{ message_id: number }>;
  deleteMessage(chatId: number | string, messageId: number): Promise<unknown>;
  answerCallbackQuery(queryId: string): Promise<unknown>;
  editMessageMedia(
    media: Record<string, unknown>,
    options?: Record<string, unknown>,
  ): Promise<unknown>;
  editMessageText(text: string, options?: Record<string, unknown>): Promise<unknown>;
  editMessageReplyMarkup(
    markup: InlineKeyboardMarkup,
    options?: Record<string, unknown>,
  ): Promise<unknown>;
  setMyCommands?(commands: Array<{ command: string; description: string }>): Promise<unknown>;
  setWebHook?(url: string): Promise<unknown>;
  on?(event: string, listener: (...args: never[]) => unknown): unknown;
};

export type CacheLike = {
  getSessions(date: string, theaterId?: string): SessionDayCache | null;
  getUpcoming(theaterId?: string): UpcomingCache | null;
  mergeMovies(movies: Record<string, MovieStatic>): number | void;
  setSessions(
    date: string,
    sessions: Session[],
    fetchedAt: string,
    theaterId?: string,
  ): Promise<void>;
  setUpcoming?(items: UpcomingItem[], fetchedAt: string, theaterId?: string): Promise<void>;
  getAllMovies(): Record<string, MovieStatic>;
  load?(): Promise<void>;
};

export type TelegramChat = {
  id: number;
  type?: string;
};

export type TelegramUser = {
  id: number;
  is_bot?: boolean;
  first_name?: string;
  username?: string;
};

export type TelegramMessage = {
  message_id: number;
  from?: TelegramUser;
  chat: TelegramChat;
  date?: number;
  text?: string;
  photo?: Array<{ file_id: string }>;
};

export type TelegramCallbackQuery = {
  id: string;
  from?: TelegramUser;
  message?: TelegramMessage;
  data?: string;
};

export type TelegramUpdate = {
  update_id?: number;
  message?: TelegramMessage;
  callback_query?: TelegramCallbackQuery;
};

export type LambdaHttpResult = {
  statusCode: number;
  headers: Record<string, string>;
  body: string;
};

export type ApiGatewayEvent = {
  body?: string | TelegramUpdate | null;
};

export type IngressoImage = { type?: string; url?: string };
export type IngressoTag = string | { name: string };
export type IngressoSessionType = { name?: string; alias?: string };
export type IngressoRawSession = {
  id: string;
  time: string;
  price?: number | null;
  room?: string;
  types?: IngressoSessionType[];
  siteURL?: string;
};
export type IngressoSessionGroup = { sessions?: IngressoRawSession[] };
export type IngressoRawMovie = {
  id: number;
  title: string;
  originalTitle?: string;
  urlKey?: string;
  duration?: number | string;
  contentRating?: string;
  ratingDetails?: { color?: string };
  genres?: string[];
  distributor?: string;
  images?: IngressoImage[];
  trailers?: Array<{ url?: string }>;
  completeTags?: IngressoTag[];
  tags?: IngressoTag[];
  isReexhibition?: boolean;
  inPreSale?: boolean;
  sessionTypes?: IngressoSessionGroup[];
  rooms?: IngressoSessionGroup[];
  siteURLByTheater?: string;
  siteURL?: string;
};
export type IngressoDateEntry = {
  date: string;
  dateFormatted?: string;
  dayOfWeek?: string;
  movies?: IngressoRawMovie[];
};

export function emptyCache(): CacheData {
  return { movies: {}, sessions: {}, upcoming: {}, moviesUpdatedAt: null };
}

export function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}
