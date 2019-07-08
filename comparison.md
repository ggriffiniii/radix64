# A running list of the primary differences with base64 crate

 * Configs are different zero sized types, that implement a Config trait.
 * There is an AVX2 optimized encoder/decoder for the non custom types.
 * Custom alphabets are supported (though you lose AVX2 optimization).
 * encode/decode are methods on the config rather than free functions that
   accept the config.
 * The _buffer variants of encode and decode accept a buffer and return a slice
   (&str when encoding, &[u8] when decoding) into the buffer. The buffer does not need to be (and
   for best performance should not be) cleared. This minimizes reinitializing
   the buffer to extend it for input. For this reason the _buffer variants have
   higher performance than the base64 versions and should be the primary
   functions used, rarely needing the _slice versions.
 * EncodeWriter accepts a writer by value, rather than &mut reference. This is
   preferred because there's a blanket impl for anything that implements Write
   to also implement Write for a &mut reference to it. This gives flexibility to
   the caller to decide whether ownership should be passed into the EncodeWriter
   or not. DecodeReader behaves the same way.
 * DecodeReader exists.

