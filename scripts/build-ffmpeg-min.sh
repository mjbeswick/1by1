#!/bin/sh
# Build a minimal FFmpeg into .ffmpeg-min: decoders and demuxers for the formats 1by1 reads,
# no encoders, no network, no external libraries. make-app.sh picks it up when present.
# Needs an unpacked FFmpeg source tree: FFMPEG_SRC=/path/to/ffmpeg-source ./scripts/build-ffmpeg-min.sh
set -e
cd "$(dirname "$0")/.."
: "${FFMPEG_SRC:?set FFMPEG_SRC to an unpacked FFmpeg source tree}"
PREFIX="$PWD/.ffmpeg-min"
cd "$FFMPEG_SRC"
./configure --prefix="$PREFIX" --enable-shared --disable-static \
    --disable-everything --disable-programs --disable-doc --disable-debug \
    --disable-network --disable-autodetect --disable-avdevice --disable-avfilter \
    --disable-x86asm --disable-iconv --enable-swresample --enable-protocol=file \
    --enable-demuxer=matroska,ogg,flac,mov,wav,aiff,mp3,caf,aac,ac3 \
    --enable-parser=aac,ac3,flac,mpegaudio,opus,vorbis \
    --enable-decoder=aac,mp3float,mp2float,flac,vorbis,opus,alac,ac3,pcm_s8,pcm_u8,pcm_s16le,pcm_s16be,pcm_s24le,pcm_s24be,pcm_s32le,pcm_s32be,pcm_f32le,pcm_f32be,pcm_f64le,pcm_f64be,pcm_alaw,pcm_mulaw
make -j8
make install
echo "built $PREFIX"
