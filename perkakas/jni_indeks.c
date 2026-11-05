/* Turunkan indeks slot JNINativeInterface_ dari jni.h milik JDK.
   Indeks yang meleset = memanggil fungsi lain dengan tanda tangan lain,
   yaitu proses mati tanpa pesan. Jadi ini tidak boleh ditebak. */
#include <jni.h>
#include <stddef.h>
#include <stdio.h>
#define SLOT(f) printf("%-28s %zu\n", #f, offsetof(struct JNINativeInterface_, f)/sizeof(void*))
int main(void) {
  /* yang sudah dipakai — untuk memastikan penurunan ini cocok dengan tabel yang ada */
  SLOT(GetVersion); SLOT(FindClass); SLOT(GetObjectClass); SLOT(GetMethodID);
  SLOT(CallObjectMethodA); SLOT(ExceptionOccurred); SLOT(ExceptionClear);
  SLOT(DeleteLocalRef); SLOT(GetStaticMethodID); SLOT(CallStaticObjectMethodA);
  SLOT(CallStaticBooleanMethodA); SLOT(CallStaticByteMethodA); SLOT(CallStaticCharMethodA);
  SLOT(CallStaticShortMethodA); SLOT(CallStaticIntMethodA); SLOT(CallStaticLongMethodA);
  SLOT(CallStaticFloatMethodA); SLOT(CallStaticDoubleMethodA); SLOT(CallStaticVoidMethodA);
  SLOT(NewStringUTF); SLOT(GetStringUTFChars); SLOT(ReleaseStringUTFChars);
  printf("--- baru ---\n");
  SLOT(NewObjectA); SLOT(NewGlobalRef); SLOT(DeleteGlobalRef);
  SLOT(CallBooleanMethodA); SLOT(CallByteMethodA); SLOT(CallCharMethodA);
  SLOT(CallShortMethodA); SLOT(CallIntMethodA); SLOT(CallLongMethodA);
  SLOT(CallFloatMethodA); SLOT(CallDoubleMethodA); SLOT(CallVoidMethodA);
  SLOT(GetFieldID); SLOT(GetObjectField); SLOT(GetBooleanField); SLOT(GetByteField);
  SLOT(GetCharField); SLOT(GetShortField); SLOT(GetIntField); SLOT(GetLongField);
  SLOT(GetFloatField); SLOT(GetDoubleField);
  SLOT(GetStaticFieldID); SLOT(GetStaticObjectField); SLOT(GetStaticBooleanField);
  SLOT(GetStaticByteField); SLOT(GetStaticCharField); SLOT(GetStaticShortField);
  SLOT(GetStaticIntField); SLOT(GetStaticLongField); SLOT(GetStaticFloatField);
  SLOT(GetStaticDoubleField);
  SLOT(IsInstanceOf); SLOT(GetArrayLength); SLOT(NewObjectArray);
  SLOT(GetObjectArrayElement); SLOT(SetObjectArrayElement);
  SLOT(NewIntArray); SLOT(NewDoubleArray); SLOT(NewLongArray); SLOT(NewBooleanArray);
  SLOT(GetIntArrayElements); SLOT(ReleaseIntArrayElements);
  SLOT(GetDoubleArrayElements); SLOT(ReleaseDoubleArrayElements);
  SLOT(ExceptionDescribe);
  return 0;
}
